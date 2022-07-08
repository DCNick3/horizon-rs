use arcstr::ArcStr;
use codespan_reporting::diagnostic::{Diagnostic, Label};
use derivative::Derivative;
use itertools::Itertools;
use std::collections::btree_map::Entry;
use std::collections::BTreeMap;
use std::ops::Range;
use std::sync::Arc;

pub type Span = Range<usize>;
type Result<T> = std::result::Result<T, Vec<Diagnostic<usize>>>;

/// Types are those things that have a known byte representations
/// Usually they will be sent as-is via the wire (put into the payload), but sometimes (structs with sf::LargeData marker) they would be sent as buffers instead
///
/// They can be included in structures
#[derive(Debug, Clone, Derivative)]
#[derivative(PartialEq)]
pub enum NominalType {
    Int(IntType),
    Bool,
    F32,
    Bytes {
        size: u64,
        alignment: u64,
    },
    Unknown {
        size: Option<u64>,
    },
    TypeName {
        name: ArcStr,
        #[derivative(PartialEq = "ignore")]
        reference_location: Span,
    },
}

impl NominalType {
    pub fn resolve_with_depth(
        &self,
        file_id: usize,
        context: &BTreeMap<ArcStr, TypeWithName>,
        depth_limit: usize,
    ) -> Result<StructuralType> {
        Ok(match self {
            &NominalType::Int(i) => StructuralType::Int(i),
            NominalType::Bool => StructuralType::Bool,
            NominalType::F32 => StructuralType::F32,
            &NominalType::Bytes { size, alignment } => StructuralType::Bytes { size, alignment },
            &NominalType::Unknown { size } => StructuralType::Unknown { size },
            NominalType::TypeName {
                name,
                reference_location,
            } => {
                if let Some(t) = context.get(name) {
                    match t {
                        TypeWithName::TypeAlias(t) => {
                            if depth_limit == 0 {
                                return Err(vec![Diagnostic::error()
                                    .with_message("Resolve recursion limit exceeded")
                                    .with_labels(vec![Label::primary(
                                        file_id,
                                        t.location.clone(),
                                    )])]);
                            }

                            let ty = t
                                .referenced_type
                                .resolve_with_depth(file_id, context, depth_limit - 1)
                                .map_err(|e| {
                                    e.into_iter()
                                        .map(|e| {
                                            e.with_labels(vec![
                                                Label::secondary(
                                                    file_id,
                                                    reference_location.clone(),
                                                )
                                                .with_message(format!(
                                                    "While resolving type named `{}`",
                                                    name
                                                )),
                                                Label::secondary(file_id, t.location.clone())
                                                    .with_message("Defined as a typedef"),
                                            ])
                                        })
                                        .collect::<Vec<_>>()
                                })?;
                            ty
                        }
                        TypeWithName::StructDef(s) => StructuralType::Struct(s.clone()),
                        TypeWithName::EnumDef(e) => StructuralType::Enum(e.clone()),
                        TypeWithName::BitflagsDef(b) => StructuralType::Bitflags(b.clone()),
                    }
                } else {
                    return Err(vec![Diagnostic::error()
                        .with_message(format!("Could not resolve type named `{}`", name))
                        .with_labels(vec![Label::primary(
                            file_id,
                            reference_location.clone(),
                        )])]);
                }
            }
        })
    }

    pub fn resolve(
        &self,
        file_id: usize,
        context: &BTreeMap<ArcStr, TypeWithName>,
    ) -> Result<StructuralType> {
        self.resolve_with_depth(file_id, context, 16)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum StructuralType {
    Int(IntType),
    Bool,
    F32,
    Bytes { size: u64, alignment: u64 },
    Unknown { size: Option<u64> },
    Struct(Arc<Struct>),
    Enum(Arc<Enum>),
    Bitflags(Arc<Bitflags>),
}

impl StructuralType {
    pub fn is_sized(&self) -> bool {
        match self {
            StructuralType::Unknown { size: None } => false,
            _ => true,
        }
    }

    pub fn size(&self) -> u64 {
        todo!()
    }

    pub fn alignment(&self) -> u64 {
        todo!()
    }

    pub fn preferred_transfer_mode(&self) -> BufferTransferMode {
        todo!()
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum HandleTransferType {
    Move,
    Copy,
}

#[derive(Debug, PartialEq, Clone)]
pub enum BufferTransferMode {
    MapAlias,
    Pointer,
    AutoSelect,
}

#[derive(Debug, PartialEq, Clone)]
pub enum BufferExtraAttrs {
    None,
    AllowNonSecure,
    AllowNonDevice,
}

#[derive(Debug, PartialEq, Clone)]
pub struct BufferType {
    pub direction: Direction,
    pub transfer_mode: BufferTransferMode,
    pub extra_attrs: BufferExtraAttrs,
}

impl BufferType {
    pub fn try_from_id(_id: u64) -> anyhow::Result<Self> {
        todo!()
        // Ok(Self {})
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Direction {
    In,
    Out,
}

/// Everything that can be sent or received using IPC
/// Includes stuff like PID descriptors, objects and handles
///
/// They can't be included in structures
///
/// TODO: bikeshed on the name
#[derive(Debug, PartialEq, Clone)]
pub enum Value {
    /// sf::ClientProcessId
    ClientProcessId,

    /// T
    In(NominalType),
    /// sf::Out<T>
    Out(NominalType),

    /// sf::SharedPointer<T>
    InObject(ArcStr),
    /// sf::Out<sf::SharedPointer<T>>
    OutObject(Option<ArcStr>),

    /// sf::CopyHandle
    /// sf::MoveHandle
    InHandle(HandleTransferType),
    /// sf::OutCopyHandle
    /// sf::OutMoveHandle
    OutHandle(HandleTransferType),

    /// sf::InArray
    /// sf::InMapAliasArray
    /// sf::InPointerArray
    /// sf::InAutoSelectArray
    InArray(NominalType, Option<BufferTransferMode>),
    /// sf::OutArray
    /// sf::OutMapAliasArray
    /// sf::OutPointerArray
    /// sf::OutAutoSelectArray
    OutArray(NominalType, Option<BufferTransferMode>),

    /// sf::InBuffer
    /// sf::InMapAliasBuffer
    /// sf::InPointerBuffer
    /// sf::InAutoSelectBuffer
    /// sf::InNonSecureBuffer
    /// sf::InNonDeviceBuffer
    /// sf::InNonSecureAutoSelectBuffer
    InBuffer(BufferTransferMode, BufferExtraAttrs),
    /// sf::OutBuffer
    /// sf::OutMapAliasBuffer
    /// sf::OutPointerBuffer
    /// sf::OutAutoSelectBuffer
    /// sf::OutNonSecureBuffer
    /// sf::OutNonDeviceBuffer
    /// sf::OutNonSecureAutoSelectBuffer
    OutBuffer(BufferTransferMode, BufferExtraAttrs),
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum IntType {
    U8,
    U16,
    U32,
    U64,
    I8,
    I16,
    I32,
    I64,
}

impl IntType {
    pub fn is_signed(&self) -> bool {
        use IntType::*;
        match self {
            U8 | U16 | U32 | U64 => false,
            I8 | I16 | I32 | I64 => true,
        }
    }

    pub fn max_value(&self) -> u64 {
        match self {
            IntType::U8 => u8::MAX as u64,
            IntType::U16 => u16::MAX as u64,
            IntType::U32 => u32::MAX as u64,
            IntType::U64 => u64::MAX as u64,
            IntType::I8 => i8::MAX as u64,
            IntType::I16 => i16::MAX as u64,
            IntType::I32 => i32::MAX as u64,
            IntType::I64 => i64::MAX as u64,
        }
    }

    pub fn fits_u64(&self, value: u64) -> bool {
        let max_val = self.max_value();
        value <= max_val
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum StructMarker {
    LargeData,
    PrefersTransferMode(BufferTransferMode),
}

impl StructMarker {
    // TODO: make some trait or smth for formatting the SwIPC files
    pub fn display(&self) -> String {
        match self {
            StructMarker::LargeData => "sf::LargeData",
            StructMarker::PrefersTransferMode(BufferTransferMode::MapAlias) => {
                "sf::PrefersMapAliasTransferMode"
            }
            StructMarker::PrefersTransferMode(BufferTransferMode::Pointer) => {
                "sf::PrefersPointerTransferMode"
            }
            StructMarker::PrefersTransferMode(BufferTransferMode::AutoSelect) => {
                "sf::PrefersAutoSelectTransferMode"
            }
        }
        .to_string()
    }
}

#[derive(Debug, Clone, Derivative)]
#[derivative(PartialEq)]
pub struct Struct {
    pub name: ArcStr,
    pub is_large_data: bool,
    pub preferred_transfer_mode: Option<BufferTransferMode>,
    pub fields: Vec<(ArcStr, NominalType)>,
    #[derivative(PartialEq = "ignore")]
    pub location: Span,
}

impl Struct {
    pub fn try_new(
        name: ArcStr,
        fields: Vec<(ArcStr, NominalType)>,
        markers: Vec<StructMarker>,
        location: Span,
    ) -> Result<Self> {
        let is_large_data = markers
            .iter()
            .find(|&v| v == &StructMarker::LargeData)
            .is_some();
        let preferred_transfer_mode = markers
            .iter()
            .filter_map(|v| match v {
                StructMarker::PrefersTransferMode(mode) => Some(mode),
                _ => None,
            })
            .at_most_one()
            .map_err(|_| {
                vec![Diagnostic::error()
                    .with_message("No more that one transfer mode preference marker must be used")
                    .with_notes(vec![format!(
                        "Found the following preference markers: {}",
                        markers
                            .iter()
                            .filter_map(|v| match v {
                                StructMarker::PrefersTransferMode(mode) =>
                                    Some(StructMarker::PrefersTransferMode(mode.clone()).display()),
                                _ => None,
                            })
                            .join(", ")
                    )])]
            })?
            .cloned();

        Ok(Self {
            name,
            is_large_data,
            preferred_transfer_mode,
            fields,
            location,
        })
    }

    pub fn typecheck(
        &self,
        file_id: usize,
        context: &BTreeMap<ArcStr, TypeWithName>,
    ) -> Result<()> {
        let mut diags = Vec::new();
        for (name, ty) in self.fields.iter() {
            match ty.resolve(file_id, context) {
                Ok(t) => {
                    if !t.is_sized() {
                        diags.push(
                            Diagnostic::error()
                                .with_message(format!("Use of unsized type in field `{}`", name))
                                .with_labels(vec![Label::primary(file_id, self.location.clone())]),
                        );
                    }
                }
                Err(e) => diags.extend(e.into_iter().map(|e| {
                    e.with_labels(vec![Label::secondary(file_id, self.location.clone())
                        .with_message(format!(
                            "In field `{}` of struct `{}`",
                            name, self.name
                        ))])
                })),
            }
        }

        if diags.is_empty() {
            Ok(())
        } else {
            Err(diags)
        }
    }
}

#[derive(Debug, Clone, Derivative)]
#[derivative(PartialEq)]
pub struct Enum {
    pub name: ArcStr,
    pub base_type: IntType,
    pub arms: Vec<(ArcStr, u64)>,
    #[derivative(PartialEq = "ignore")]
    pub location: Span,
}

impl Enum {
    pub fn typecheck(
        &self,
        file_id: usize,
        _context: &BTreeMap<ArcStr, TypeWithName>,
    ) -> Result<()> {
        let mut diags = Vec::new();

        let mut arm_values: BTreeMap<u64, &ArcStr> = BTreeMap::new();

        for (name, value) in self.arms.iter() {
            if !self.base_type.fits_u64(*value) {
                diags.push(
                    Diagnostic::error()
                        .with_message(format!(
                            "Value {} of enum arm `{}` does not fit into type {:?}",
                            value, name, self.base_type
                        ))
                        .with_labels(vec![Label::primary(file_id, self.location.clone())]),
                )
            }

            match arm_values.entry(*value) {
                Entry::Vacant(e) => {
                    e.insert(name);
                }
                Entry::Occupied(o) => diags.push(
                    Diagnostic::error()
                        .with_message(format!(
                            "Value {} of enum arm `{}` is the same as value in arm `{}`",
                            value,
                            name,
                            o.get()
                        ))
                        .with_labels(vec![Label::primary(file_id, self.location.clone())]),
                ),
            }
        }

        if diags.is_empty() {
            Ok(())
        } else {
            Err(diags)
        }
    }
}

#[derive(Debug, Clone, Derivative)]
#[derivative(PartialEq)]
pub struct Bitflags {
    pub name: ArcStr,
    pub base_type: IntType,
    pub arms: Vec<(ArcStr, u64)>,
    #[derivative(PartialEq = "ignore")]
    pub location: Span,
}

impl Bitflags {
    pub fn typecheck(
        &self,
        file_id: usize,
        _context: &BTreeMap<ArcStr, TypeWithName>,
    ) -> Result<()> {
        let mut diags = Vec::new();
        for (name, value) in self.arms.iter() {
            if !self.base_type.fits_u64(*value) {
                diags.push(
                    Diagnostic::error()
                        .with_message(format!(
                            "Value {} of bitfield arm `{}` does not fit into type {:?}",
                            value, name, self.base_type
                        ))
                        .with_labels(vec![Label::primary(file_id, self.location.clone())]),
                )
            }
        }

        if diags.is_empty() {
            Ok(())
        } else {
            Err(diags)
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Command {
    // TODO: do we want to support multiple versions & version requirements at all?
    pub id: u32,
    pub name: ArcStr,
    // those define both in and out arguments
    pub arguments: Vec<(Option<ArcStr>, Arc<Value>)>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Interface {
    pub name: ArcStr,
    pub sm_names: Vec<ArcStr>,
    pub commands: Vec<Command>,
}

#[derive(Debug, Clone, Derivative)]
#[derivative(PartialEq)]
pub struct TypeAlias {
    pub name: ArcStr,
    pub referenced_type: NominalType,
    #[derivative(PartialEq = "ignore")]
    pub location: Span,
}

#[derive(Debug, PartialEq, Clone)]
pub enum IpcFileItem {
    TypeAlias(Arc<TypeAlias>),
    StructDef(Arc<Struct>),
    EnumDef(Arc<Enum>),
    BitflagsDef(Arc<Bitflags>),
    InterfaceDef(Arc<Interface>),
}

#[derive(Debug, PartialEq, Clone)]
pub enum TypeWithName {
    TypeAlias(Arc<TypeAlias>),
    StructDef(Arc<Struct>),
    EnumDef(Arc<Enum>),
    BitflagsDef(Arc<Bitflags>),
}

impl TypeWithName {
    pub fn typecheck(
        &self,
        file_id: usize,
        context: &BTreeMap<ArcStr, TypeWithName>,
    ) -> Result<()> {
        match self {
            TypeWithName::TypeAlias(t) => {
                t.referenced_type.resolve(file_id, context)?;
                Ok(())
            }
            TypeWithName::StructDef(s) => s.typecheck(file_id, context),
            TypeWithName::EnumDef(e) => e.typecheck(file_id, context),
            TypeWithName::BitflagsDef(b) => b.typecheck(file_id, context),
        }
    }
}

impl TypeWithName {
    pub fn name(&self) -> &ArcStr {
        match self {
            TypeWithName::TypeAlias(a) => &a.name,
            TypeWithName::StructDef(s) => &s.name,
            TypeWithName::EnumDef(e) => &e.name,
            TypeWithName::BitflagsDef(b) => &b.name,
        }
    }

    pub fn location(&self) -> Span {
        match self {
            TypeWithName::TypeAlias(a) => a.location.clone(),
            TypeWithName::StructDef(s) => s.location.clone(),
            TypeWithName::EnumDef(e) => e.location.clone(),
            TypeWithName::BitflagsDef(b) => b.location.clone(),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct IpcFile {
    items: Vec<IpcFileItem>,
    resolved_type_names: BTreeMap<ArcStr, StructuralType>,
    resolved_interfaces: BTreeMap<ArcStr, Arc<Interface>>,
}

impl IpcFile {
    pub fn try_new(file_id: usize, items: Vec<IpcFileItem>) -> Result<Self> {
        let mut named_types = BTreeMap::new();

        let mut diagnostics = Vec::new();

        let mut try_add_named_type = |ty: TypeWithName| {
            let name = ty.name().clone();
            let e = named_types.entry(name.clone());
            match e {
                Entry::Vacant(vac) => {
                    vac.insert(ty);
                }
                Entry::Occupied(occ) => {
                    diagnostics.push(
                        Diagnostic::error()
                            .with_message(format!("Multiple definitions of type `{}`", name))
                            .with_labels(vec![
                                Label::primary(file_id, ty.location()),
                                Label::secondary(file_id, occ.get().location()).with_message(
                                    format!("Previous definition of type `{}`", name),
                                ),
                            ]),
                    );
                }
            }
        };

        for item in items.iter() {
            match item {
                IpcFileItem::TypeAlias(alias) => {
                    try_add_named_type(TypeWithName::TypeAlias(alias.clone()));
                }
                IpcFileItem::StructDef(s) => {
                    try_add_named_type(TypeWithName::StructDef(s.clone()));
                }
                IpcFileItem::EnumDef(e) => {
                    try_add_named_type(TypeWithName::EnumDef(e.clone()));
                }
                IpcFileItem::BitflagsDef(b) => {
                    try_add_named_type(TypeWithName::BitflagsDef(b.clone()));
                }
                IpcFileItem::InterfaceDef(_) => {}
            }
        }

        for (_, named_type) in named_types.iter() {
            if let Err(e) = named_type.typecheck(file_id, &named_types) {
                diagnostics.extend(e);
            }
        }

        if diagnostics.is_empty() {
            todo!()
        } else {
            Err(diagnostics)
        }
    }
}
