use anyhow::{anyhow, Context};
use heck::ToSnakeCase;
use lazy_static::lazy_static;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use regex::Regex;
use scraper::{ElementRef, Html, Selector};
use semver::Version;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::str::FromStr;
use strum::EnumString;

// revision of SVC switchbrew page
// can be found by clicking on "View history" button and examining the link to the latest revision
// The link has a form https://switchbrew.org/w/index.php?title=SVC&oldid={revision}
const REVISION: u32 = 11749;
lazy_static! {
    static ref LAST_SUPPORTED_VERSION: Version = Version::parse("13.0.0").unwrap();
}

const SYSCALL_IGNORE_LIST: &[&str] = &[
    "CallSecureMonitor", // it has a problematic argument list. And it's not like it is useful for most applications (I think?)
    "ContinueDebugEvent", // [3.0.0+] and friends in argument names
];

lazy_static! {
    static ref VERSION_RANGE_REGEX: Regex =
        Regex::new(r"^\[(\d+\.\d+\.\d+)-(\d+\.\d+\.\d+)\]$").unwrap();
    static ref MIN_VERSION_REGEX: Regex = Regex::new(r"^\[(\d+\.\d+\.\d+)\+\]$").unwrap();
}

lazy_static! {
    static ref NAME_LINK_REGEX: Regex = Regex::new("^<a href=\"#(\\w+)\">(\\w+)</a>$").unwrap();
    static ref PLAIN_NAME_REGEX: Regex = Regex::new("^[a-zA-Z][a-zA-Z0-9]*$").unwrap();
}

#[derive(Debug)]
enum VersionReq {
    Any,
    MinVersion(Version),
    VersionRange { min: Version, max: Version },
}

impl Display for VersionReq {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            VersionReq::Any => {
                write!(f, "")
            }
            VersionReq::MinVersion(min) => {
                write!(f, "[{}+]", min)
            }
            VersionReq::VersionRange { min, max } => {
                write!(f, "[{}-{}]", min, max)
            }
        }
    }
}

fn parse_id(id: &str) -> anyhow::Result<(VersionReq, u32)> {
    if id.starts_with('0') {
        u32::from_str_radix(
            id.strip_prefix("0x").context("Stripping the 0x prefix")?,
            16,
        )
        .ok()
        .with_context(|| format!("Parsing syscall id string {}", id))
        .map(|id| (VersionReq::Any, id))
    } else {
        let (req, id) = id
            .split_once(' ')
            .context("Splitting the id string with some version requirements")?;
        let id = u32::from_str_radix(
            id.strip_prefix("0x").context("Stripping the 0x prefix")?,
            16,
        )
        .ok()
        .with_context(|| format!("Parsing syscall id string {}", id))?;

        let req = if let Some(c) = MIN_VERSION_REGEX.captures(req) {
            let min_version = c.get(1).unwrap().as_str();
            let min_version = Version::parse(min_version)
                .with_context(|| format!("Parsing version {}", min_version))?;

            VersionReq::MinVersion(min_version)
        } else if let Some(c) = VERSION_RANGE_REGEX.captures(req) {
            let min_version = c.get(1).unwrap().as_str();
            let max_version = c.get(2).unwrap().as_str();

            let min_version = Version::parse(min_version)
                .with_context(|| format!("Parsing version {}", min_version))?;
            let max_version = Version::parse(max_version)
                .with_context(|| format!("Parsing version {}", max_version))?;

            VersionReq::VersionRange {
                min: min_version,
                max: max_version,
            }
        } else {
            return Err(anyhow!("Unknown version requirement syntax: {}", req));
        };

        Ok((req, id))
    }
}

#[derive(Debug)]
struct Syscall {
    /// Syscall number
    pub id: u32,
    /// Name of the syscall
    pub name: String,
    /// HOS version requirements for this syscall
    #[allow(unused)] // TODO: use this to codegen docs
    pub version_req: VersionReq,
    /// Info on in & out params for this syscall (as they are described on switchbrew)
    pub params_info: Option<ParamsInfo>,
    /// raw html from switchbrew in section for this syscall
    #[allow(unused)] // TODO: use this to codegen docs
    pub raw_docs: Option<String>,
}

fn split_sections(content: ElementRef) -> anyhow::Result<HashMap<String, String>> {
    let mut res = HashMap::new();
    let mut current_section: Option<(String, String)> = None;

    for child in content.children() {
        if let Some(child) = ElementRef::wrap(child) {
            if &child.value().name.local == "h2" {
                if let Some((name, content)) = &mut current_section {
                    let mut new_content = String::new();

                    std::mem::swap(&mut new_content, content);

                    res.insert(name.clone(), new_content);
                }

                let section_name = child.text().next().unwrap().trim();

                current_section = Some((section_name.to_string(), String::new()));
            }
        }

        if let Some((_, contents)) = &mut current_section {
            if let Some(el) = ElementRef::wrap(child) {
                *contents += &el.html();
            } else if let Some(text) = child.value().as_text() {
                *contents += &text.text;
            } else if let Some(_) = child.value().as_comment() {
                // ignore
            } else {
                todo!("Dunno what to do with this node")
            }
        }
    }

    Ok(res)
}

#[derive(Debug)]
struct ParamsInfo {
    pub in_params: Vec<SyscallParam>,
    pub out_params: Vec<SyscallParam>,
}

#[derive(Debug, EnumString)]
enum Register {
    X0,
    X1,
    X2,
    X3,
    X4,
    X5,
    X6,
    X7,
    W0,
    W1,
    W2,
    W3,
    W4,
    W5,
    W6,
    W7,
}

impl Register {
    pub fn is_64bit(&self) -> bool {
        use Register::*;
        match self {
            X0 | X1 | X2 | X3 | X4 | X5 | X6 | X7 => true,
            W0 | W1 | W2 | W3 | W4 | W5 | W6 | W7 => false,
        }
    }
}

#[derive(Debug)]
enum ParamKind {
    Result,
    Integer32,
    Integer64,
    Pointer,
}

impl ParamKind {
    pub fn as_tokens(&self) -> TokenStream {
        match self {
            ParamKind::Result => quote!(ErrorCode),
            ParamKind::Integer32 => quote!(u32),
            ParamKind::Integer64 => quote!(u64),
            ParamKind::Pointer => quote!(*const u8),
        }
    }
    pub fn as_raw_tokens(&self) -> TokenStream {
        match self {
            ParamKind::Result | ParamKind::Integer32 => quote!(u32),
            ParamKind::Integer64 => quote!(u64),
            ParamKind::Pointer => quote!(*const u8),
        }
    }
}

#[derive(Debug)]
enum ParamDirection {
    In,
    Out,
}

#[derive(Debug)]
struct SyscallParam {
    pub register: Register,
    pub kind: ParamKind,
    pub name: String,
}

fn parse_syscall_param(
    register: &str,
    ty: &str,
    name: &str,
) -> anyhow::Result<Option<(ParamDirection, SyscallParam)>> {
    if register.trim().ends_with("None") {
        return Ok(None);
    }
    let (direction, register) = register
        .split_once(' ')
        .context("Splitting register specification by space")?;

    let direction = if direction == "(In)" {
        ParamDirection::In
    } else if direction == "(Out)" {
        ParamDirection::Out
    } else {
        todo!("Unknown param direction: {}", direction)
    };

    let register: Register = Register::from_str(register).context("Unknown register name")?;

    let name = name.to_snake_case();

    let kind = if ty == "#Result" {
        ParamKind::Result
    } else if ty.ends_with('*') {
        assert!(
            register.is_64bit(),
            "Non-64 bit register used for a pointer: type = {}, register = {:?}",
            ty,
            register,
        );
        ParamKind::Pointer
    } else if register.is_64bit() {
        ParamKind::Integer64
    } else {
        ParamKind::Integer32
    };

    Ok(Some((
        direction,
        SyscallParam {
            register,
            kind,
            name,
        },
    )))
}

fn parse_syscall_params(html: &str) -> anyhow::Result<ParamsInfo> {
    let mut params = Vec::new();

    let mut handle_param = |i: usize, register: &str, ty: &str, name: &str| -> anyhow::Result<()> {
        let ty = if ty.is_empty() {
            "".to_string()
        } else {
            Html::parse_fragment(ty)
                .root_element()
                .text()
                .next()
                .context("Converting type element to text")?
                .to_string()
        };

        // sometimes parameters in switchbrew don't have names
        let name = if name.is_empty() {
            format!("unnamed_{}", i + 1)
        } else {
            name.to_string()
        };

        params.push(
            parse_syscall_param(register, &ty, &name)
                .with_context(|| format!("Parsing parameter {}", name))?,
        );

        Ok(())
    };

    if let Some(table) = table_extract::Table::find_by_headers(html, &["Argument", "Type", "Name"])
    {
        for (i, row) in table.iter().enumerate() {
            let register = row.get("Argument").unwrap();
            let ty = row.get("Type").unwrap();
            let name = row.get("Name").unwrap();

            handle_param(i, register, ty, name)?;
        }
    } else if let Some(table) =
        table_extract::Table::find_by_headers(html, &["Argument64", "Argument32", "Type", "Name"])
    {
        for (i, row) in table.iter().enumerate() {
            let register = row.get("Argument64").unwrap();
            let ty = row.get("Type").unwrap();
            let name = row.get("Name").unwrap_or(""); // special case for GetDebugFutureThreadInfo... So, unnamed params are possible

            handle_param(i, register, ty, name)?;
        }
    } else {
        todo!("Unknown parameter table form")
    };

    let mut res = ParamsInfo {
        in_params: Vec::new(),
        out_params: Vec::new(),
    };

    for (dir, param) in params.into_iter().flatten() {
        match dir {
            ParamDirection::In => res.in_params.push(param),
            ParamDirection::Out => res.out_params.push(param),
        }
    }

    Ok(res)
}

fn get_syscalls() -> anyhow::Result<Vec<Syscall>> {
    let url = format!(
        "https://switchbrew.org/w/index.php?title=SVC&oldid={}",
        REVISION
    );

    let html = reqwest::blocking::get(url).context("Getting switchbrew SVC page")?;
    let html = html.text().context("Getting switchbrew SVC page text")?;

    let table =
        table_extract::Table::find_by_headers(&html, &["ID", "Return Type", "Name", "Arguments"])
            .context("Finding syscall table on the page")?;

    let html = Html::parse_fragment(&html);

    let document_content = html
        .select(&Selector::parse(".mw-parser-output").unwrap())
        .next()
        .unwrap();

    let sections = split_sections(document_content)?;

    let mut res = Vec::new();

    for row in table.iter() {
        let id = row.get("ID").context("Getting ID from table row")?;
        let name = row.get("Name").context("Getting Name from table row")?;

        let name = if PLAIN_NAME_REGEX.is_match(name.trim()) {
            // name can be either text
            name.trim()
        } else {
            // or a link to a section describing the syscall
            let c = NAME_LINK_REGEX.captures(name).with_context(|| {
                format!("Matching name link to extract syscall name from '{}'", name)
            })?;
            assert_eq!(
                c.get(1).unwrap().as_str(),
                c.get(2).unwrap().as_str(),
                "The anchor name should be the same as the syscall name"
            );
            c.get(1).unwrap().as_str()
        };

        if SYSCALL_IGNORE_LIST.contains(&name) {
            continue;
        }

        let (version_req, id) = parse_id(id)?;

        let raw_docs = sections.get(name).cloned();

        let params_info = raw_docs
            .as_ref()
            .map(|docs| parse_syscall_params(docs))
            .map_or(Ok(None), |v| v.map(Some))
            .with_context(|| format!("Parsing parameters info for syscall {}", name))?;

        res.push(Syscall {
            id,
            name: name.to_string(),
            version_req,
            params_info,
            raw_docs,
        });
    }

    Ok(res)
}

/// Gets the rustfmt path to rustfmt the generated code.
fn rustfmt_path() -> anyhow::Result<PathBuf> {
    if let Ok(rustfmt) = std::env::var("RUSTFMT") {
        return Ok(rustfmt.into());
    }
    Ok(which::which("rustfmt")?)
}
fn rustfmt_generated_string(source: &str) -> anyhow::Result<String> {
    let rustfmt = rustfmt_path()?;
    let mut cmd = Command::new(&*rustfmt);

    cmd.stdin(Stdio::piped()).stdout(Stdio::piped());

    let mut child = cmd.spawn()?;
    let mut child_stdin = child.stdin.take().unwrap();
    let mut child_stdout = child.stdout.take().unwrap();

    let source = source.to_owned();

    // Write to stdin in a new thread, so that we can read from stdout on this
    // thread. This keeps the child from blocking on writing to its stdout which
    // might block us from writing to its stdin.
    let stdin_handle = ::std::thread::spawn(move || {
        let _ = child_stdin.write_all(source.as_bytes());
        source
    });

    let mut output = vec![];
    std::io::copy(&mut child_stdout, &mut output)?;

    let status = child.wait()?;
    let source = stdin_handle.join().expect(
        "The thread writing to rustfmt's stdin doesn't do \
             anything that could panic",
    );

    match String::from_utf8(output) {
        Ok(output) => match status.code() {
            Some(0) => Ok(output),
            Some(2) => Err(anyhow!("Rustfmt parsing errors.")),
            Some(3) => Ok(output),
            _ => Err(anyhow!("Internal rustfmt error")),
        },
        _ => Ok(source),
    }
}

fn make_ident(name: &str) -> TokenStream {
    // keywords list based on https://doc.rust-lang.org/reference/keywords.html
    let ident = match name {
        "abstract" | "as" | "become" | "box" | "break" | "const" | "continue" | "crate" | "do"
        | "else" | "enum" | "extern" | "false" | "final" | "fn" | "for" | "if" | "impl" | "in"
        | "let" | "loop" | "macro" | "match" | "mod" | "move" | "mut" | "override" | "priv"
        | "pub" | "ref" | "return" | "static" | "struct" | "super" | "trait" | "true" | "type"
        | "typeof" | "unsafe" | "unsized" | "use" | "virtual" | "where" | "while" | "yield"
        | "try" | "async" | "await" | "dyn" => format_ident!("r#{}", name),
        "Self" | "self" => format_ident!("{}_", name),
        "_" => format_ident!("unused"),
        _ => format_ident!("{}", name),
    };

    quote!(#ident)
}

fn codegen(syscalls: &Vec<Syscall>) -> anyhow::Result<String> {
    let mut ts = quote! {
        //! Note: auto-generated file
        //! It is generated by horizon-svc-codegen by parsing the switchbrew wiki
        #![allow(unused)] // some syscalls will obviously be not used
        #![allow(clippy::redundant_field_names)] // this complicates codegen

        use core::arch::asm;
        use horizon_error::ErrorCode;
    };

    for Syscall {
        id,
        name,
        params_info,
        ..
    } in syscalls
    {
        if let Some(ParamsInfo {
            in_params,
            out_params,
        }) = params_info
        {
            let asm_str = format!("svc {:#04x}", id);

            let result_struct_name = make_ident(&format!("{}Result", name));

            let function_name = make_ident(&name.to_snake_case());

            let in_names = in_params
                .iter()
                .map(|p| make_ident(&p.name))
                .collect::<Vec<_>>();
            let in_exprs = in_params
                .iter()
                .map(|p| {
                    let ident = make_ident(&p.name);
                    match p.kind {
                        ParamKind::Result => quote!(#ident.repr()),
                        _ => ident,
                    }
                })
                .collect::<Vec<_>>();
            let in_types = in_params
                .iter()
                .map(|p| p.kind.as_tokens())
                .collect::<Vec<_>>();
            let in_registers = in_params
                .iter()
                .map(|p| format!("{:?}", p.register).to_ascii_lowercase())
                .collect::<Vec<_>>();

            let out_names = out_params
                .iter()
                .map(|p| make_ident(&p.name))
                .collect::<Vec<_>>();
            let out_exprs = out_params
                .iter()
                .map(|p| {
                    let ident = make_ident(&p.name);
                    match p.kind {
                        ParamKind::Result => quote!(ErrorCode::new_unchecked(#ident)),
                        _ => ident,
                    }
                })
                .collect::<Vec<_>>();
            let out_types = out_params
                .iter()
                .map(|p| p.kind.as_tokens())
                .collect::<Vec<_>>();
            let out_raw_types = out_params
                .iter()
                .map(|p| p.kind.as_raw_tokens())
                .collect::<Vec<_>>();
            let out_registers = out_params
                .iter()
                .map(|p| format!("{:?}", p.register).to_ascii_lowercase())
                .collect::<Vec<_>>();

            ts.extend([quote! {
                pub struct #result_struct_name {
                    #(pub #out_names: #out_types,)*
                }

                #[inline(always)]
                #[must_use]
                pub unsafe fn #function_name(#(#in_names: #in_types),*) -> #result_struct_name {
                    #(let #out_names: #out_raw_types;)*

                    asm!(#asm_str, #(in(#in_registers) #in_exprs,)* #(lateout(#out_registers) #out_names,)*);

                    #result_struct_name {
                        #(#out_names: #out_exprs,)*
                    }
                }
            }]);
        }
    }

    rustfmt_generated_string(&ts.to_string())
}

fn main() -> anyhow::Result<()> {
    let syscalls = get_syscalls()?;

    let generated = codegen(&syscalls)?;

    let output_path = PathBuf::from("horizon-svc/src/raw.rs");

    std::fs::write(output_path,generated).context("Cannot write output file. Please make sure you are running it from the cargo workspace root")?;

    Ok(())
}
