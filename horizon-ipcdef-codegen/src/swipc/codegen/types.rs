use crate::swipc::codegen::import_in;
use crate::swipc::model::{IntType, Namespace};
use crate::swipc::{
    codegen,
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
    let name = codegen::make_ident(s.name.ident());
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
                $(for f in s.fields.iter() join (,) {
                    $(f.name.as_str()): $(make_nominal_type(namespace, &f.ty))
                })
            }

            _comment_!($(quoted(size_assert_comment)));
            const _: fn() = || { let _ = ::core::mem::transmute::<$name, [u8; $size]>; };

            _blank_!();
        },
    );
}

#[cfg(test)]
mod tests {
    use crate::swipc::codegen::types::gen_struct;
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
}
