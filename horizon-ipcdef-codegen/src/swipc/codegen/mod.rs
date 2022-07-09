use crate::swipc::model::{Namespace, NamespacedIdent};
use arcstr::ArcStr;
use genco::fmt::Indentation;
use genco::lang::rust::Tokens;
use genco::lang::{rust, Rust};
use genco::quote;
use rust_format::{Formatter, PostProcess};
use std::collections::BTreeMap;
use std::sync::Arc;

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
        let imp = rust::import(relative_mod_name, import_item.ident().as_str());

        quote!($imp)
    }
}

fn filename_for_namespace(namespace: &Namespace) -> String {
    let mut r = String::new();

    for part in namespace.iter() {
        r.push_str(part);
        r.push('/');
    }

    r.push_str("mod.rs");

    r
}

struct TokenStorage {
    storage: BTreeMap<Arc<Vec<ArcStr>>, Tokens>,
}

impl TokenStorage {
    pub fn new() -> Self {
        Self {
            storage: BTreeMap::new(),
        }
    }

    pub fn push(&mut self, namespace: Namespace, tokens: Tokens) {
        self.storage.entry(namespace).or_default().append(tokens);
    }

    pub fn to_file_string(self) -> anyhow::Result<BTreeMap<String, String>> {
        // TODO: add "mod" directives

        self.storage
            .into_iter()
            .map(|(ns, tok)| {
                let name = filename_for_namespace(&ns);

                let mut w = genco::fmt::FmtWriter::new(String::new());
                let fmt =
                    genco::fmt::Config::from_lang::<Rust>().with_indentation(Indentation::Space(4));
                let mut formatter = w.as_formatter(&fmt);
                let config = rust::Config::default();
                tok.format_file(&mut formatter, &config)?;

                let contents = w.into_inner();

                let formatter = make_formatter();
                let contents = formatter.format_str(contents)?;

                Ok((name, contents))
            })
            .collect::<anyhow::Result<_>>()
    }
}

fn make_formatter() -> impl rust_format::Formatter {
    let config = rust_format::Config::new_str().post_proc(PostProcess::ReplaceMarkersAndDocBlocks);

    rust_format::PrettyPlease::from_config(config)
}
