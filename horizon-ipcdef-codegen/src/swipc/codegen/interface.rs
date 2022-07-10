use crate::swipc::codegen::types::make_nominal_type;
use crate::swipc::codegen::{import_in, make_ident, TokenStorage};
use crate::swipc::model::{CodegenContext, Command, Interface, Namespace, NamespacedIdent, Value};
use arcstr::ArcStr;
use convert_case::{Case, Casing};
use genco::lang::rust::Tokens;
use genco::prelude::*;

fn make_interface_reference(current_namespace: &Namespace, name: &NamespacedIdent) -> Tokens {
    import_in(current_namespace, name)
}

fn make_session_handle_ref() -> Tokens {
    let imp = rust::import("horizon_ipc::cmif", "SessionHandleRef");

    quote!($imp)
}

fn make_session_handle() -> Tokens {
    let imp = rust::import("horizon_ipc::cmif", "SessionHandle");

    quote!($imp)
}

fn make_result() -> Tokens {
    let imp = rust::import("horizon_error", "Result");

    quote!($imp)
}

fn make_raw_handle() -> Tokens {
    let imp = rust::import("horizon_ipc", "RawHandle");

    quote!($imp)
}

fn gen_command_in(namespace: &Namespace, tok: &mut Tokens, ctx: &CodegenContext, c: &Command) {
    let mut args = Vec::new();
    let mut results = Vec::new();

    let mut should_pass_pid = false;

    for (name, arg) in c.arguments.iter() {
        let name = name
            .as_ref()
            .cloned()
            .unwrap_or_else(|| ArcStr::from(format!("unnamed_{}", args.len() + 1)));

        match arg.as_ref() {
            Value::ClientProcessId => {
                should_pass_pid = true;
                continue;
            }
            Value::In(ty) => {
                // pass In values by value
                let ty = make_nominal_type(namespace, ty);

                args.push((
                    name,
                    quote! {
                        $ty
                    },
                ));
            }
            Value::Out(ty) => {
                // pass Out values by mutable reference (the impl will write the result back)
                // an unfortunate consequence is that the value should be initialized before the call =(
                // ignore for now, but kinda an important part of an API
                let ty = make_nominal_type(namespace, ty);
                results.push((
                    name,
                    quote! {
                        $ty
                    },
                ));
            }
            Value::InObject(_, _) => {
                todo!()
            }
            Value::OutObject(interface_name, _) => {
                if let Some(interface_name) = interface_name {
                    results.push((
                        name,
                        quote! {
                            $(make_interface_reference(namespace, interface_name))
                        },
                    ))
                } else {
                    todo!("Handling return of unknown object type")
                }
            }
            Value::InHandle(_) => {
                todo!()
            }
            Value::OutHandle(_) => {
                // we just emit a RawHandle out param no matter what
                results.push((
                    name,
                    quote! {
                        $(make_raw_handle())
                    },
                ));
            }
            Value::InArray(_, _) => {
                todo!()
            }
            Value::OutArray(_, _) => {
                todo!()
            }
            Value::InBuffer(_, _) => {
                todo!()
            }
            Value::OutBuffer(_, _) => {
                todo!()
            }
        };
    }

    let return_type = if let [(_, res)] = results.as_slice() {
        quote!($res) as Tokens
    } else {
        // TODO: doing this we lose names. This is not __that__ bad, but kinda meh...
        quote! {
            (
                $(for (_, ty) in results.iter() join (,) => $ty)
            )
        }
    };

    // we expect command names in PascalCase, but convert them to snake_case when converting to rust
    let name = c.name.to_case(Case::Snake);
    quote_in! { *tok =>
        fn $name(
            $(for (name, ty) in args join (,) => $(name.as_str()): $ty)
        ) -> $(make_result())<$return_type> {
            todo!("Command codegen")
        }
    }
}

pub fn gen_interface(tok: &mut TokenStorage, ctx: &CodegenContext, i: &Interface) {
    let name = make_ident(i.name.ident());
    let name = &name;
    let namespace = i.name.namespace();

    if i.is_domain {
        todo!("Domain interfaces codegen")
    }

    let mut commands_impl = Tokens::new();
    for command in i.commands.iter() {
        gen_command_in(namespace, &mut commands_impl, ctx, command);
    }

    tok.push(
        namespace.clone(),
        quote! {
            pub struct $name {
                // the generated interface object owns the session handle!
                handle: $(make_session_handle()),
            }

            impl $name {
                $commands_impl
            }
        },
    );
}

#[cfg(test)]
mod tests {
    use crate::swipc::codegen::interface::gen_interface;
    use crate::swipc::codegen::TokenStorage;
    use crate::swipc::model::{IpcFileItem, TypecheckedIpcFile};
    use crate::swipc::tests::{parse_typechecked_ipc_file, unwrap_parse};
    use indoc::indoc;
    use itertools::Itertools;

    #[test]
    fn simple_interface() {
        let s = r#"
            interface IHelloInterface {
                [0] HelloCommand();
                [1] HelloCommand1(u8 input_1, sf::Out<u32>, u16 input_2, sf::Out<sf::Bytes<0x20>> output_2);
            }
        "#;

        let file: TypecheckedIpcFile = unwrap_parse(s, parse_typechecked_ipc_file);

        let item = file.iter_items().next().unwrap();
        // TODO: add an into_struct method or smth
        let i = match item {
            IpcFileItem::InterfaceDef(i) => i,
            _ => unreachable!(),
        };

        let mut ts = TokenStorage::new();

        gen_interface(&mut ts, file.context(), i);

        let (_, res) = ts
            .to_file_string()
            .unwrap()
            .into_iter()
            .exactly_one()
            .unwrap();

        println!("{}", res);

        assert_eq!(
            res,
            indoc! {r#"
                #![allow(clippy::all)]
                use horizon_error::Result;
                use horizon_ipc::cmif::SessionHandle;
                pub struct IHelloInterface {
                    handle: SessionHandle,
                }
                impl IHelloInterface {
                    fn hello_command() -> Result<()> {
                        todo!("Command codegen")
                    }
                    fn hello_command_1(
                        input_1: u8,
                        unnamed_2: &mut u32,
                        input_2: u16,
                        output_2: &mut [u8; 32],
                    ) -> Result<()> {
                        todo!("Command codegen")
                    }
                }
            "#}
        )
    }
}
