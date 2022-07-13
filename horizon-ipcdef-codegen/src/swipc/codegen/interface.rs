use crate::swipc::codegen::types::make_nominal_type;
use crate::swipc::codegen::{import_in, make_ident, TokenStorage};
use crate::swipc::diagnostics::Span;
use crate::swipc::model::{
    BufferExtraAttrs, BufferTransferMode, CodegenContext, Command, Direction, HandleTransferType,
    IntType, Interface, Namespace, NamespacedIdent, NominalType, Struct, StructField, Value,
};
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

fn make_maybe_uninit() -> Tokens {
    let imp = rust::import("core::mem", "MaybeUninit");

    quote!($imp)
}

enum BufferSource {
    /// We have a byte slice in scope that should be converted to a buffer
    ByteSlice(ArcStr),
    /// We have a local variable of type MaybeUninit<some IPC type> that should have reference taken and converted to a buffer
    TypedUninitVariable(ArcStr),
    /// We have a reference to some IPC type in scope that should be converted to a buffer
    TypedReference(ArcStr),
    /// We have a slice of an IPC type in scope that should be converted to a buffer
    TypedSlice(ArcStr),
}

struct Buffer {
    source: BufferSource,
    direction: Direction,
    transfer_mode: BufferTransferMode,
    fixed_size: bool,
    extra_attrs: BufferExtraAttrs,
}

enum RawDataInSource {
    Local,
    PidPlaceholder,
}

struct RawDataIn {
    name: ArcStr,
    source: RawDataInSource,
    ty: NominalType,
}

struct RawDataOut {
    name: ArcStr,
    ty: NominalType,
}

struct HandleIn {
    name: ArcStr,
    transfer_type: HandleTransferType,
}

struct HandleOut {
    name: ArcStr,
    transfer_type: HandleTransferType,
    transform: HandleTransformType,
}

enum HandleTransformType {
    RawValue,
    Interface(NamespacedIdent),
}

fn make_raw_data_struct(
    namespace: &Namespace,
    ctx: &CodegenContext,
    direction: Direction,
    items: impl Iterator<Item = (ArcStr, NominalType)>,
) -> Tokens {
    let name = NamespacedIdent::new(namespace.clone(), ArcStr::from(format!("{:?}", direction)));

    let s = Struct::try_new(
        name.clone(),
        items
            .map(|(name, ty)| StructField {
                name,
                ty,
                location: Span::default(),
            })
            .collect(),
        vec![],
        Span::default(),
    )
    .unwrap();

    let size = s.layout(ctx).size();

    let name = name.ident().as_str();

    quote! {
        #[repr(C)]
        struct $name {
            $(for field in s.fields join (,) {
                $(field.name.as_str()): $(make_nominal_type(namespace, &field.ty))
            })
        }

        let _ = ::core::mem::transmute::<$name, [u8; $size]>;
    }
}

fn make_raw_data_in(namespace: &Namespace, ctx: &CodegenContext, data: &[RawDataIn]) -> Tokens {
    if data.is_empty() {
        (quote! {
            let data_in = ();
        } as Tokens)
    } else if let [data] = data {
        quote! {
            let data_in = $(match data.source {
                RawDataInSource::PidPlaceholder =>
                    0u64,

                RawDataInSource::Local =>
                    $(data.name.as_str()),
            });
        }
    } else {
        quote! {
            $(make_raw_data_struct(
                namespace,
                ctx,
                Direction::In,
                data
                    .iter()
                    .map(|d| (d.name.clone(), d.ty.clone()))
            ))

            let data_in: In = In {
                $(for data in data join (,) {
                    $(match data.source {
                        RawDataInSource::PidPlaceholder =>
                            $(data.name.as_str()): 0,

                        RawDataInSource::Local =>
                            $(data.name.as_str()),
                    })
                })
            };
        }
    }
}

fn gen_command_in(
    namespace: &Namespace,
    tok: &mut Tokens,
    ctx: &CodegenContext,
    c: &Command,
    is_domain: bool,
) {
    let mut args = Vec::new();
    let mut results = Vec::new();
    let mut uninit_vars = Vec::new();

    let mut buffers = Vec::new();
    let mut raw_data_in = Vec::new();
    let mut raw_data_out = Vec::new();
    let mut handles_in = Vec::new();
    let mut handles_out = Vec::new();

    let mut should_pass_pid = false;

    for (name, arg) in c.arguments.iter() {
        let name = name
            .as_ref()
            .cloned()
            .unwrap_or_else(|| ArcStr::from(format!("unnamed_{}", args.len() + 1)));

        match arg.as_ref() {
            Value::ClientProcessId => {
                should_pass_pid = true;
                raw_data_in.push(RawDataIn {
                    name: arcstr::literal!("_pid_placeholder"),
                    source: RawDataInSource::PidPlaceholder,
                    ty: NominalType::Int(IntType::U64),
                });
                continue;
            }
            Value::In(ty) => {
                let struct_ty = ctx.resolve_type(ty);
                let is_large_data = struct_ty.is_large_data();

                if is_large_data {
                    buffers.push(Buffer {
                        source: BufferSource::TypedReference(name.clone()),
                        direction: Direction::In,
                        transfer_mode: struct_ty.preferred_transfer_mode(),
                        extra_attrs: BufferExtraAttrs::None,
                        fixed_size: true,
                    });
                } else {
                    raw_data_in.push(RawDataIn {
                        name: name.clone(),
                        source: RawDataInSource::Local,
                        ty: ty.clone(),
                    })
                }

                // pass In values by value
                let ty_tok = make_nominal_type(namespace, ty);

                args.push((
                    name,
                    quote! {
                        $(if is_large_data { &$ty_tok } else { $ty_tok } )
                    },
                ));
            }
            Value::Out(ty) => {
                let struct_ty = ctx.resolve_type(ty);
                let is_large_data = struct_ty.is_large_data();

                let ty_tok = make_nominal_type(namespace, ty);
                let ty_tok = &ty_tok;

                if is_large_data {
                    uninit_vars.push((name.clone(), quote!($ty_tok)));
                    buffers.push(Buffer {
                        source: BufferSource::TypedReference(name.clone()),
                        direction: Direction::Out,
                        transfer_mode: struct_ty.preferred_transfer_mode(),
                        extra_attrs: BufferExtraAttrs::None,
                        fixed_size: true,
                    });
                } else {
                    raw_data_out.push(RawDataOut {
                        name: name.clone(),
                        ty: ty.clone(),
                    })
                }
                results.push((
                    name,
                    quote! {
                        $ty_tok
                    },
                ));
            }
            Value::InObject(_, _) => {
                assert!(is_domain, "Input objects supported only in domain requests");
                todo!()
            }
            Value::OutObject(interface_name, _) => {
                if is_domain {
                    todo!("Domains not implemented")
                }

                if let Some(interface_name) = interface_name {
                    let interface = ctx.resolve_interface(interface_name);

                    if interface.is_domain {
                        todo!("Domains not implemented")
                    }

                    handles_out.push(HandleOut {
                        name: name.clone(),
                        transfer_type: HandleTransferType::Copy,
                        transform: HandleTransformType::Interface(interface_name.clone()),
                    });

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
            &Value::InHandle(transfer_type) => {
                // TODO: we probably want to distinguish between Move and Copy handles here
                // by taking in owning or referencing handle types
                handles_in.push(HandleIn {
                    name: name.clone(),
                    transfer_type,
                });

                args.push((
                    name,
                    quote! {
                        $(make_raw_handle())
                    },
                ));
            }
            &Value::OutHandle(transfer_type) => {
                handles_out.push(HandleOut {
                    name: name.clone(),
                    transfer_type,
                    transform: HandleTransformType::RawValue,
                });

                results.push((
                    name,
                    quote! {
                        $(make_raw_handle())
                    },
                ));
            }
            Value::InArray(ty, transfer_mode) => {
                let struct_ty = ctx.resolve_type(ty);

                buffers.push(Buffer {
                    source: BufferSource::TypedSlice(name.clone()),
                    direction: Direction::In,
                    transfer_mode: transfer_mode
                        .unwrap_or_else(|| struct_ty.preferred_transfer_mode()),
                    fixed_size: struct_ty.is_large_data(),
                    extra_attrs: BufferExtraAttrs::None,
                });

                args.push((
                    name,
                    quote! {
                        &[$(make_nominal_type(namespace, ty))]
                    },
                ));
            }
            Value::OutArray(ty, transfer_mode) => {
                let struct_ty = ctx.resolve_type(ty);

                buffers.push(Buffer {
                    source: BufferSource::TypedSlice(name.clone()),
                    direction: Direction::Out,
                    transfer_mode: transfer_mode
                        .unwrap_or_else(|| struct_ty.preferred_transfer_mode()),
                    fixed_size: struct_ty.is_large_data(),
                    extra_attrs: BufferExtraAttrs::None,
                });

                args.push((
                    name,
                    quote! {
                        &mut [$(make_nominal_type(namespace, ty))]
                    },
                ));
            }
            &Value::InBuffer(transfer_mode, extra_attrs) => {
                buffers.push(Buffer {
                    source: BufferSource::ByteSlice(name.clone()),
                    direction: Direction::In,
                    transfer_mode,
                    fixed_size: false,
                    extra_attrs,
                });

                args.push((
                    name,
                    quote! {
                        &[u8]
                    },
                ));
            }
            &Value::OutBuffer(transfer_mode, extra_attrs) => {
                buffers.push(Buffer {
                    source: BufferSource::ByteSlice(name.clone()),
                    direction: Direction::Out,
                    transfer_mode,
                    fixed_size: false,
                    extra_attrs,
                });

                args.push((
                    name,
                    quote! {
                        &mut [u8]
                    },
                ));
            }
        };
    }

    // sort raw data by alignment, because.... This is ABI
    // they tried to make an optimal packing, but this is suboptimal, lol
    raw_data_in.sort_by_cached_key(|d| d.ty.layout(&ctx).alignment());
    raw_data_out.sort_by_cached_key(|d| d.ty.layout(&ctx).alignment());

    assert!(buffers.len() <= 8, "Methods must take in <= 8 Buffers");
    assert!(handles_in.len() <= 8, "Methods must take in <= 8 Handles");
    assert!(handles_out.len() <= 8, "Methods must output <= 8 Handles");

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
        pub fn $name(
            $(for (name, ty) in args join (,) => $(name.as_str()): $ty)
        ) -> $(make_result())<$return_type> {

            // defines a data_in variable
            $(make_raw_data_in(namespace, ctx, &raw_data_in))


            // TODO: process output
            $(if raw_data_out.is_empty() {
            } else {
                $(make_raw_data_struct(
                    namespace,
                    ctx,
                    Direction::Out,
                    raw_data_out
                        .iter()
                        .map(|d| (d.name.clone(), d.ty.clone()))
                ))
            })

            $(for (name, ty) in uninit_vars {
                let $(name.as_str()) = $(make_maybe_uninit())::<$ty>::uninit();
            })

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
        gen_command_in(namespace, &mut commands_impl, ctx, command, i.is_domain);
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
