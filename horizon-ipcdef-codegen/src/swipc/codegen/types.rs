use crate::swipc::codegen::{import_in, make_ident};
use crate::swipc::model::{Bitflags, Enum, IntType, Namespace, TypeAlias};
use crate::swipc::{
    codegen::{TokenStorage, Tokens},
    model::{CodegenContext, NominalType, Struct},
};
use genco::prelude::*;

fn make_int_type(t: IntType) -> Tokens {
    match t {
        IntType::U8 => quote!(u8),
        IntType::U16 => quote!(u16),
        IntType::U32 => quote!(u32),
        IntType::U64 => quote!(u64),
        IntType::I8 => quote!(i8),
        IntType::I16 => quote!(i16),
        IntType::I32 => quote!(i32),
        IntType::I64 => quote!(i64),
    }
}

fn make_nominal_type(current_namespace: &Namespace, t: &NominalType) -> Tokens {
    match t {
        &NominalType::Int(t) => make_int_type(t),
        NominalType::Bool => quote!(bool),
        NominalType::F32 => quote!(f32),
        &NominalType::Bytes { size, alignment } => {
            if alignment != 1 {
                todo!("Aligned bytes")
            }
            quote!([u8; $size])
        }
        NominalType::Unknown { .. } => {
            todo!("Handle 'Unknown' type in codegen")
        }
        NominalType::TypeName { name, .. } => import_in(current_namespace, name),
    }
}

fn gen_struct(tok: &mut TokenStorage, ctx: &CodegenContext, s: &Struct) {
    let name = make_ident(s.name.ident());
    let name = &name;
    let namespace = s.name.namespace();

    let layout = s.layout(ctx);
    let size = layout.size();

    let size_assert_comment = format!(
        "Static size check for {} (expect {} bytes)",
        s.name.ident(),
        size
    );

    tok.push(
        namespace.clone(),
        quote! {
            $(if s.is_large_data { #[doc = " This struct is marked with sf::LargeData"] })
            #[repr(C)]
            pub struct $name {
                $(for f in s.fields.iter() {
                    $(make_ident(&f.name)): $(make_nominal_type(namespace, &f.ty)),
                })
            }

            _comment_!($(quoted(size_assert_comment)));
            const _: fn() = || { let _ = ::core::mem::transmute::<$name, [u8; $size]>; };

            _blank_!();
        },
    );
}

fn gen_enum(tok: &mut TokenStorage, _ctx: &CodegenContext, e: &Enum) {
    let name = make_ident(e.name.ident());
    let namespace = e.name.namespace();

    let base_type = make_int_type(e.base_type);

    tok.push(
        namespace.clone(),
        quote! {
            #[repr($base_type)]
            pub enum $name {
                $(for arm in e.arms.iter() {
                    $(make_ident(&arm.name)) = $(arm.value),
                })
            }
        },
    );
}

fn gen_bitflags(tok: &mut TokenStorage, _ctx: &CodegenContext, b: &Bitflags) {
    let name = make_ident(b.name.ident());
    let namespace = b.name.namespace();

    let base_type = make_int_type(b.base_type);

    let bitflags_macro = rust::import("bitflags", "bitflags");

    tok.push(
        namespace.clone(),
        quote! {
            // NOTE: currently prettyplease ignores anything inside the macro call
            // This is expected, but it leaves all the bitflags completely unformatted!
            // Furthermore, even if we feed some formatted input to it it butchers it, discarding any formatting information
            // I don't think it's possible to get nicer output using prettyplease without ditching bitflags! macro
            $bitflags_macro! {
                pub struct $name : $base_type {
                    $(for arm in b.arms.iter() {
                        const $(make_ident(&arm.name)) = $(format!("{:#x}", arm.value));
                    })
                }
            }
        },
    );
}

fn gen_type_alias(tok: &mut TokenStorage, _ctx: &CodegenContext, a: &TypeAlias) {
    let name = make_ident(a.name.ident());
    let namespace = a.name.namespace();

    let ty = make_nominal_type(namespace, &a.referenced_type);

    tok.push(
        namespace.clone(),
        quote! {
            type $name = $ty;
        },
    );
}

#[cfg(test)]
mod tests {
    use crate::swipc::codegen::types::{gen_bitflags, gen_enum, gen_struct, gen_type_alias};
    use crate::swipc::codegen::{import_in, TokenStorage};
    use crate::swipc::model::{IpcFile, IpcFileItem, NamespacedIdent};
    use crate::swipc::tests::{parse_ipc_file, unwrap_parse};
    use indoc::indoc;
    use itertools::Itertools;

    #[test]
    fn relative_import() {
        let current_item = NamespacedIdent::parse("a::b::c::A").unwrap();
        let import_item = NamespacedIdent::parse("a::b::d::B").unwrap();

        let tok = import_in(current_item.namespace(), &import_item);

        let file = tok.to_file_vec().unwrap();

        println!("{}", file.join("\n"));

        assert_eq!(file, vec!["use super::d::::B;", "", "B"])
    }

    #[test]
    fn relative_import_same_module() {
        let current_item = NamespacedIdent::parse("a::b::c::A").unwrap();
        let import_item = NamespacedIdent::parse("a::b::c::B").unwrap();

        let tok = import_in(current_item.namespace(), &import_item);

        let file = tok.to_file_vec().unwrap();

        println!("{}", file.join("\n"));

        assert_eq!(file, vec!["B"])
    }

    #[test]
    fn simple_struct() {
        let s = r#"
            struct HelloStruct : sf::LargeData {
                /// This is a doc-comment (allowed only in certain places)
                u8 aaaa;
                /// 7 bytes of padding here (u64 should be 8-byte aligned)
                u64 padded;
                u16 bbbb;
                /// 2 bytes of padding here (u32 should be 4-byte aligned)
                u32 cccc;
                u8 ddd;
                /// 7 bytes of padding here (because the whole structure size should be 8-byte aligned to be able to be placed in an array)
            };
        "#;

        let file: IpcFile = unwrap_parse(s, parse_ipc_file);

        let item = file.iter_items().next().unwrap();
        // TODO: add an into_struct method or smth
        let s = match item {
            IpcFileItem::StructDef(s) => s,
            _ => unreachable!(),
        };

        let mut ts = TokenStorage::new();

        gen_struct(&mut ts, file.context(), s);

        let (_, res) = ts
            .to_file_string()
            .unwrap()
            .into_iter()
            .exactly_one()
            .unwrap();

        println!("{}", res);

        assert_eq!(
            res,
            indoc! {"
                /// This struct is marked with sf::LargeData
                #[repr(C)]
                pub struct HelloStruct {
                    aaaa: u8,
                    padded: u64,
                    bbbb: u16,
                    cccc: u32,
                    ddd: u8,
                }
                // Static size check for HelloStruct (expect 32 bytes)
                const _: fn() = || {
                    let _ = ::core::mem::transmute::<HelloStruct, [u8; 32]>;
                };

            "}
        )
    }

    #[test]
    fn simple_enum() {
        let s = r#"
            enum HelloEnum : u16 {
                HelloArm = 1,
                HelloRam = 65535,
                Lol = 2,
            };
        "#;

        let file: IpcFile = unwrap_parse(s, parse_ipc_file);

        let item = file.iter_items().next().unwrap();
        // TODO: add an into_struct method or smth
        let e = match item {
            IpcFileItem::EnumDef(s) => s,
            _ => unreachable!(),
        };

        let mut ts = TokenStorage::new();

        gen_enum(&mut ts, file.context(), e);

        let (_, res) = ts
            .to_file_string()
            .unwrap()
            .into_iter()
            .exactly_one()
            .unwrap();

        println!("{}", res);

        assert_eq!(
            res,
            indoc! {"
                #[repr(u16)]
                pub enum HelloEnum {
                    HelloArm = 1,
                    HelloRam = 65535,
                    Lol = 2,
                }
            "}
        )
    }

    #[test]
    fn simple_bitflags() {
        let s = r#"
            bitflags HelloEnum : u8 {
                Arm1 = 0x1,
                Arm2 = 0x2,
                Arm12 = 0x3,
            };
        "#;

        let file: IpcFile = unwrap_parse(s, parse_ipc_file);

        let item = file.iter_items().next().unwrap();
        // TODO: add an into_struct method or smth
        let b = match item {
            IpcFileItem::BitflagsDef(s) => s,
            _ => unreachable!(),
        };

        let mut ts = TokenStorage::new();

        gen_bitflags(&mut ts, file.context(), b);

        let (_, res) = ts
            .to_file_string()
            .unwrap()
            .into_iter()
            .exactly_one()
            .unwrap();

        println!("{}", res);

        assert_eq!(
            res,
            indoc! {"
                use bitflags::bitflags;
                bitflags! {
                    pub struct HelloEnum : u8 { const Arm1 = 0x1; const Arm2 = 0x2; const Arm12 = 0x3; }
                }
            "}
        )
    }

    #[test]
    fn simple_alias() {
        let s = r#"
            type HelloAlias = sf::Bytes<0x1000>;
        "#;

        let file: IpcFile = unwrap_parse(s, parse_ipc_file);

        let item = file.iter_items().next().unwrap();
        // TODO: add an into_struct method or smth
        let a = match item {
            IpcFileItem::TypeAlias(s) => s,
            _ => unreachable!(),
        };

        let mut ts = TokenStorage::new();

        gen_type_alias(&mut ts, file.context(), a);

        let (_, res) = ts
            .to_file_string()
            .unwrap()
            .into_iter()
            .exactly_one()
            .unwrap();

        println!("{}", res);

        assert_eq!(
            res,
            indoc! {"
                type HelloAlias = [u8; 4096];
            "}
        )
    }
}
