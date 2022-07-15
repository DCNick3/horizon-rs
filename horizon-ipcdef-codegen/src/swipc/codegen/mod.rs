use crate::swipc::codegen::interface::gen_interface;
use crate::swipc::codegen::types::{gen_bitflags, gen_enum, gen_struct, gen_type_alias};
use crate::swipc::model::{
    CodegenContext, IpcFileItem, Namespace, NamespacedIdent, TypecheckedIpcFile,
};
use anyhow::Context;
use arcstr::ArcStr;
use genco::fmt::Indentation;
use genco::lang::rust::Tokens;
use genco::lang::{rust, Rust};
use genco::quote;
use itertools::Itertools;
use rust_format::{Formatter, PostProcess};
use sequence_trie::SequenceTrie;
use std::collections::BTreeMap;
use std::sync::Arc;

pub mod interface;
pub mod types;

type Item = genco::tokens::Item<Rust>;

fn make_ident(ident: &ArcStr) -> Tokens {
    use once_cell::sync::Lazy;
    use regex::Regex;
    static IDENT_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[a-zA-Z_]\w*$").unwrap());

    assert!(IDENT_REGEX.is_match(ident));

    let id = Item::Literal(ident.as_str().into());

    quote!($id)
}

fn import_in(current_namespace: &Namespace, import_item: &NamespacedIdent) -> Tokens {
    let mut current_it = current_namespace.iter().peekable();
    let mut import_it = import_item.iter_namespaces().peekable();

    loop {
        if let Some(current) = current_it.peek() {
            if let Some(import) = import_it.peek() {
                if current == import {
                    current_it.next().unwrap();
                    import_it.next().unwrap();
                } else {
                    break;
                }
            } else {
                break;
            }
        } else {
            break;
        }
    }

    let mut relative_mod_name = String::new();
    for _ in current_it {
        relative_mod_name.push_str("super::");
    }
    for part in import_it {
        relative_mod_name.push_str(part);
        relative_mod_name.push_str("::");
    }

    if relative_mod_name.is_empty() {
        // it's in the same module, just use it as-is, lol
        let id = Item::Literal(import_item.ident().as_str().into());

        quote!($id)
    } else {
        // remove the last "::"
        relative_mod_name.remove(relative_mod_name.len() - 1);
        relative_mod_name.remove(relative_mod_name.len() - 1);

        let imp = rust::import(relative_mod_name, import_item.ident().as_str());

        quote!($imp)
    }
}

fn filename_for_namespace(namespace: &Namespace, is_directory: bool) -> String {
    let mut r = String::new();

    for part in namespace.iter() {
        r.push_str(part);
        r.push('/');
    }

    if is_directory {
        r.push_str("mod.rs");
    } else {
        r.pop();
        r.push_str(".rs");
    }

    r
}

pub struct TokenStorage {
    storage: BTreeMap<Arc<Vec<ArcStr>>, Tokens>,
}

impl TokenStorage {
    pub fn new() -> Self {
        let mut res = Self {
            storage: BTreeMap::new(),
        };

        // add an dummy root file to generate at least a `mod.rs` on empty input
        res.push(Namespace::new(Vec::new()), Tokens::new());

        res
    }

    pub fn push(&mut self, namespace: Namespace, tokens: Tokens) {
        self.storage.entry(namespace).or_default().append(tokens);
    }

    pub fn to_file_string(mut self) -> anyhow::Result<BTreeMap<String, String>> {
        let namespaces_trie = {
            let mut builder = SequenceTrie::new();

            for namespace in self.storage.keys() {
                for i in 0..=namespace.len() {
                    // we want to push all base namespaces!
                    builder.insert(&namespace.as_slice()[..i], ());
                }
            }

            builder
        };

        // synthesise all intermediate modules to put "mod" directives in them
        for (namespace, ()) in namespaces_trie.iter() {
            let namespace = namespace.into_iter().map(|v| v.clone()).collect::<Vec<_>>();
            self.storage.entry(Arc::new(namespace)).or_default();
        }

        self.storage
            .into_iter()
            .map(|(ns, tok)| {
                let node = if ns.is_empty() {
                    &namespaces_trie
                } else {
                    namespaces_trie.get_node(ns.iter()).unwrap()
                };

                let child_modules = node
                    .children_with_keys()
                    .into_iter()
                    .map(|(name, _)| name.clone())
                    .sorted()
                    .collect::<Vec<_>>();

                let tok = quote! {
                    $(for module in child_modules.iter() {
                        mod $(module.as_str());
                    })

                    $tok
                };

                let should_be_directory_module = !child_modules.is_empty();
                let name = filename_for_namespace(&ns, should_be_directory_module);

                let mut w = genco::fmt::FmtWriter::new(String::new());
                let fmt =
                    genco::fmt::Config::from_lang::<Rust>().with_indentation(Indentation::Space(4));
                let mut formatter = w.as_formatter(&fmt);
                let config = rust::Config::default();
                tok.format_file(&mut formatter, &config)?;

                let contents = w.into_inner();

                let contents = if ns.is_empty() {
                    // ns is empty => we are at the root of the generated tree
                    // suppress various lints here
                    // we do it as a plain-text append because genco puts imports above everything
                    //   and it's a no-no for module-level attributes
                    // TODO: report to genco
                    format!(
                        "#![allow(non_upper_case_globals, clippy::all)]\n{}",
                        contents
                    )
                } else {
                    contents
                };

                let formatter = make_formatter();
                let contents = formatter
                    .format_str(contents)
                    .with_context(|| format!("Formatting {}", name))?;

                Ok((name, contents))
            })
            .collect::<anyhow::Result<_>>()
    }
}

fn make_formatter() -> impl rust_format::Formatter {
    let config = rust_format::Config::new_str().post_proc(PostProcess::ReplaceMarkersAndDocBlocks);

    rust_format::PrettyPlease::from_config(config)
}

pub fn gen_ipc_file(tok: &mut TokenStorage, ctx: &CodegenContext, f: &TypecheckedIpcFile) {
    for item in f.iter_items() {
        match item {
            IpcFileItem::TypeAlias(a) => gen_type_alias(tok, ctx, a),
            IpcFileItem::StructDef(s) => gen_struct(tok, ctx, s),
            IpcFileItem::EnumDef(e) => gen_enum(tok, ctx, e),
            IpcFileItem::BitflagsDef(b) => gen_bitflags(tok, ctx, b),
            IpcFileItem::InterfaceDef(i) => gen_interface(tok, ctx, i),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::swipc::codegen::{gen_ipc_file, TokenStorage};
    use crate::swipc::model::TypecheckedIpcFile;
    use crate::swipc::tests::{parse_typechecked_ipc_file, unwrap_parse};
    use indoc::indoc;

    #[test]
    fn multifile() {
        let s = r#"
            struct ns_1::Struct1 {
                ns_2::Enum1 test;
            };
            enum ns_2::Enum1 : u32 {
                Arm1 = 1,
                Arm2 = 2,
            };
            type ns_3::HelloAlias = ns_1::Struct1;
            type ns_3::nested::HelloAlias2 = ns_3::HelloAlias;
        "#;

        let file: TypecheckedIpcFile = unwrap_parse(s, parse_typechecked_ipc_file);

        let mut ts = TokenStorage::new();

        gen_ipc_file(&mut ts, file.context(), &file);

        let files = ts.to_file_string().unwrap();

        for (name, file) in files.iter() {
            println!("--- {} ---", name);
            println!("{}\n\n", file);
        }

        let expected_files = [
            (
                "mod.rs",
                indoc! {"
                    #![allow(clippy::all)]
                    mod ns_1;
                    mod ns_2;
                    mod ns_3;
                "},
            ),
            (
                "ns_1/mod.rs",
                indoc! {"
                    use super::ns_2::Enum1;
                    #[repr(C)]
                    pub struct Struct1 {
                        pub test: Enum1,
                    }
                    // Static size check for Struct1 (expect 4 bytes)
                    const _: fn() = || {
                        let _ = ::core::mem::transmute::<Struct1, [u8; 4]>;
                    };

                "},
            ),
            (
                "ns_2/mod.rs",
                indoc! {"
                    #[repr(u32)]
                    pub enum Enum1 {
                        Arm1 = 1,
                        Arm2 = 2,
                    }
                "},
            ),
            (
                "ns_3/mod.rs",
                indoc! {"
                    use super::ns_1::Struct1;
                    mod nested;
                    pub type HelloAlias = Struct1;
                "},
            ),
            (
                "ns_3/nested/mod.rs",
                indoc! {"
                    use super::HelloAlias;
                    pub type HelloAlias2 = HelloAlias;
                "},
            ),
        ]
        .into_iter()
        .map(|(n, c)| (n.to_string(), c.to_string()))
        .collect();

        assert_eq!(files, expected_files);
    }
}
