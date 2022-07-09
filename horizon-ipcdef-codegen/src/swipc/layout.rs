use crate::swipc::model::{CodegenContext, IntType, NominalType, Struct, StructuralType};

#[derive(Debug, PartialEq, Clone)]
pub struct TypeLayout {
    size: u64,
    alignment: u64,
}

impl TypeLayout {
    pub fn new(size: u64, alignment: u64) -> Self {
        assert!(alignment.is_power_of_two());
        assert!(alignment > 0);
        Self { size, alignment }
    }

    pub fn size(&self) -> u64 {
        self.size
    }

    pub fn alignment(&self) -> u64 {
        self.alignment
    }
}

impl IntType {
    pub fn layout(&self) -> TypeLayout {
        use IntType::*;
        let size = match self {
            U8 | I8 => 1,
            U16 | I16 => 2,
            U32 | I32 => 4,
            U64 | I64 => 8,
        };

        TypeLayout::new(size, size)
    }
}

impl StructuralType {
    pub fn layout(&self, context: &CodegenContext) -> TypeLayout {
        match self {
            StructuralType::Int(i) => i.layout(),
            StructuralType::Bool => TypeLayout::new(1, 1),
            StructuralType::F32 => TypeLayout::new(4, 4),
            &StructuralType::Bytes { size, alignment } => TypeLayout::new(size, alignment),
            &StructuralType::Unknown { size } => TypeLayout::new(size.unwrap(), 1),
            StructuralType::Struct(s) => s.layout(context),
            StructuralType::Enum(e) => e.base_type.layout(),
            StructuralType::Bitflags(b) => b.base_type.layout(),
        }
    }
}

impl NominalType {
    pub fn layout(&self, context: &CodegenContext) -> TypeLayout {
        self.codegen_resolve(context).layout(context)
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum FieldsLayoutItem {
    Padding(u64),
    Field(u64, usize),
}

#[derive(Debug, PartialEq, Clone)]
pub struct StructLayout {
    pub field_offsets: Vec<u64>,
    pub items: Vec<FieldsLayoutItem>,
    pub alignment: u64,
    pub size: u64,
}

impl Struct {
    #[allow(unused_assignments)]
    pub fn fields_layout(&self, context: &CodegenContext) -> StructLayout {
        let mut position = 0;

        let mut field_offsets = Vec::new();
        let mut items = Vec::new();

        let mut insert_item = |item: FieldsLayoutItem| -> u64 {
            items.push(item);

            match item {
                FieldsLayoutItem::Padding(size) => {
                    position += size;
                }
                FieldsLayoutItem::Field(size, _) => {
                    field_offsets.push(position);
                    position += size;
                }
            }

            position
        };

        let mut max_alignment = 1;

        {
            let mut position = 0;

            for (index, field) in self.fields.iter().enumerate() {
                let layout = field.ty.layout(context);

                max_alignment = std::cmp::max(max_alignment, layout.alignment);

                let required_padding =
                    (layout.alignment - (position % layout.alignment)) % layout.alignment;

                if required_padding > 0 {
                    position = insert_item(FieldsLayoutItem::Padding(required_padding));
                }

                position = insert_item(FieldsLayoutItem::Field(layout.size, index))
            }

            let required_padding = (max_alignment - (position % max_alignment)) % max_alignment;
            if required_padding > 0 {
                position = insert_item(FieldsLayoutItem::Padding(required_padding));
            }
        }

        StructLayout {
            field_offsets,
            items,
            size: position,
            alignment: max_alignment,
        }
    }

    pub fn layout(&self, context: &CodegenContext) -> TypeLayout {
        let layout = self.fields_layout(context);

        TypeLayout::new(layout.size, layout.alignment)
    }
}

#[cfg(test)]
mod tests {
    use crate::swipc::layout::StructLayout;
    use crate::swipc::model::{IpcFile, IpcFileItem};
    use crate::swipc::tests::{parse_ipc_file, unwrap_parse};

    #[test]
    fn simple_struct_layout() {
        let s = r#"
struct HelloStruct {
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

        let layout = s.fields_layout(file.context());

        println!("{:#?}", layout);

        use crate::swipc::layout::FieldsLayoutItem::{Field, Padding};
        assert_eq!(
            layout,
            StructLayout {
                field_offsets: vec![0, 8, 16, 20, 24],
                items: vec![
                    Field(1, 0),
                    Padding(7),
                    Field(8, 1),
                    Field(2, 2),
                    Padding(2),
                    Field(4, 3),
                    Field(1, 4),
                    Padding(7),
                ],
                alignment: 8,
                size: 32,
            }
        )
    }
}
