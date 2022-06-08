use anyhow::{anyhow, Context};
use heck::ToShoutySnakeCase;
use lazy_static::lazy_static;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use regex::Regex;
use semver::Version;
use std::fmt::{Display, Formatter};
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::str::FromStr;

// revision of SVC switchbrew page
// can be found by clicking on "View history" button and examining the link to the latest revision
// The link has a form https://switchbrew.org/w/index.php?title=SVC&oldid={revision}
const REVISION: u32 = 11597;
lazy_static! {
    static ref LAST_SUPPORTED_VERSION: Version = Version::parse("13.0.0").unwrap();
}

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

struct Syscall {
    pub id: u32,
    pub name: String,
    #[allow(unused)] // TODO: do come codegen for version requirements or smth
    pub version_req: VersionReq,
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

        let (version_req, id) = parse_id(id)?;

        res.push(Syscall {
            id,
            name: name.to_string(),
            version_req,
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

fn codegen(syscalls: &Vec<Syscall>) -> anyhow::Result<String> {
    let mut ts = quote! {
        //! Note: auto-generated file
        //! It is generated by horizon-svc-codegen by parsing the switchbrew wiki
        //! Therefore the numbers should be correct =)
        #![allow(unused)]
    };

    for syscall in syscalls {
        let id = syscall.id;
        let id = TokenStream::from_str(&format!("{:#04x}", id))
            .map_err(|_| anyhow!("Problem parsing hexadecimal syscall id token"))?;

        let shouty_snake_case_name = syscall.name.to_shouty_snake_case();
        let nr_ident = format_ident!("{}", shouty_snake_case_name);

        ts.extend([quote! {
            pub const #nr_ident: u32 = #id;
        }]);
    }

    rustfmt_generated_string(&ts.to_string())
}

fn main() -> anyhow::Result<()> {
    let syscalls = get_syscalls()?;

    let generated = codegen(&syscalls)?;

    let output_path = PathBuf::from("horizon-svc/src/nr.rs");

    std::fs::write(output_path,generated).context("Cannot write output file. Please make sure you are running it from the cargo workspace root")?;

    Ok(())
}
