use crate::swipc::diagnostics;
use crate::swipc::diagnostics::{DiagnosticExt, DiagnosticResultExt, Span};
use arcstr::ArcStr;
use codespan_reporting::diagnostic::Diagnostic;
use derivative::Derivative;
use itertools::Itertools;
use std::cell::RefCell;
use std::collections::btree_map::Entry;
use std::collections::BTreeMap;
use std::sync::Arc;

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
        context: &TypecheckContext,
        depth_limit: usize,
    ) -> diagnostics::Result<StructuralType> {
        Ok(match self {
            &NominalType::Int(i) => StructuralType::Int(i),
            NominalType::Bool => StructuralType::Bool,
            NominalType::F32 => StructuralType::F32,
            &NominalType::Bytes { size, alignment } => StructuralType::Bytes { size, alignment },
            &NominalType::Unknown { size } => StructuralType::Unknown { size },
            NominalType::TypeName {
                name,
                reference_location,
            } => match context.resolve_type(name, reference_location)? {
                TypeWithName::TypeAlias(t) => {
                    if depth_limit == 0 {
                        return Err(vec![Diagnostic::error()
                            .with_message("Resolve recursion limit exceeded")
                            .with_primary_label(t.location)]);
                    }

                    let ty = t
                        .referenced_type
                        .resolve_with_depth(context, depth_limit - 1)
                        .with_context(reference_location, || {
                            format!("While resolving type named `{}`", name)
                        })?;
                    ty
                }
                TypeWithName::StructDef(s) => StructuralType::Struct(s.clone()),
                TypeWithName::EnumDef(e) => StructuralType::Enum(e.clone()),
                TypeWithName::BitflagsDef(b) => StructuralType::Bitflags(b.clone()),
            },
        })
    }

    pub fn resolve(&self, context: &TypecheckContext) -> diagnostics::Result<StructuralType> {
        self.resolve_with_depth(context, 16)
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
    InObject(ArcStr, Span),
    /// sf::Out<sf::SharedPointer<T>>
    OutObject(Option<ArcStr>, Span),

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
pub struct StructField {
    pub name: ArcStr,
    pub ty: NominalType,
    #[derivative(PartialEq = "ignore")]
    pub location: Span,
}

#[derive(Debug, Clone, Derivative)]
#[derivative(PartialEq)]
pub struct Struct {
    pub name: ArcStr,
    pub is_large_data: bool,
    pub preferred_transfer_mode: Option<BufferTransferMode>,
    pub fields: Vec<StructField>,
    #[derivative(PartialEq = "ignore")]
    pub location: Span,
}

impl Struct {
    pub fn try_new(
        name: ArcStr,
        fields: Vec<StructField>,
        markers: Vec<StructMarker>,
        location: Span,
    ) -> diagnostics::Result<Self> {
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
}

#[derive(Debug, Clone, Derivative)]
#[derivative(PartialEq)]
pub struct EnumArm {
    pub name: ArcStr,
    pub value: u64,
    #[derivative(PartialEq = "ignore")]
    pub location: Span,
}

#[derive(Debug, Clone, Derivative)]
#[derivative(PartialEq)]
pub struct Enum {
    pub name: ArcStr,
    pub base_type: IntType,
    pub arms: Vec<EnumArm>,
    #[derivative(PartialEq = "ignore")]
    pub location: Span,
}

#[derive(Debug, Clone, Derivative)]
#[derivative(PartialEq)]
pub struct BitflagsArm {
    pub name: ArcStr,
    pub value: u64,
    #[derivative(PartialEq = "ignore")]
    pub location: Span,
}

#[derive(Debug, Clone, Derivative)]
#[derivative(PartialEq)]
pub struct Bitflags {
    pub name: ArcStr,
    pub base_type: IntType,
    pub arms: Vec<BitflagsArm>,
    #[derivative(PartialEq = "ignore")]
    pub location: Span,
}

#[derive(Debug, Clone, Derivative)]
#[derivative(PartialEq)]
pub struct Command {
    // TODO: do we want to support multiple versions & version requirements at all?
    pub id: u32,
    pub name: ArcStr,
    // those define both in and out arguments
    pub arguments: Vec<(Option<ArcStr>, Arc<Value>)>,
    #[derivative(PartialEq = "ignore")]
    pub location: Span,
}

#[derive(Debug, Clone, Derivative)]
#[derivative(PartialEq)]
pub struct Interface {
    pub name: ArcStr,
    pub sm_names: Vec<ArcStr>,
    pub commands: Vec<Command>,
    #[derivative(PartialEq = "ignore")]
    pub location: Span,
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
pub struct TypecheckContext {
    pub named_types: BTreeMap<ArcStr, TypeWithName>,
    pub interfaces: BTreeMap<ArcStr, Arc<Interface>>,
}

impl TypecheckContext {
    pub fn resolve_type(
        &self,
        name: &ArcStr,
        reference_location: &Span,
    ) -> diagnostics::Result<TypeWithName> {
        if let Some(t) = self.named_types.get(name) {
            Ok(t.clone())
        } else {
            return Err(vec![Diagnostic::error()
                .with_message(format!("Could not resolve type named `{}`", name))
                .with_primary_label(reference_location)]);
        }
    }

    pub fn resolve_interface(
        &self,
        name: &ArcStr,
        reference_location: &Span,
    ) -> diagnostics::Result<Arc<Interface>> {
        if let Some(t) = self.interfaces.get(name) {
            Ok(t.clone())
        } else {
            return Err(vec![Diagnostic::error()
                .with_message(format!("Could not resolve interface named `{}`", name))
                .with_primary_label(reference_location)]);
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
    pub fn try_new(items: Vec<IpcFileItem>) -> diagnostics::Result<Self> {
        let mut named_types = BTreeMap::new();
        let mut interfaces = BTreeMap::new();

        let diagnostics = RefCell::new(Vec::new());

        let mut try_add_named_type = |ty: TypeWithName| {
            let name = ty.name().clone();
            let e = named_types.entry(name.clone());
            match e {
                Entry::Vacant(vac) => {
                    vac.insert(ty);
                }
                Entry::Occupied(occ) => {
                    diagnostics.borrow_mut().push(
                        Diagnostic::error()
                            .with_message(format!("Multiple definitions of type `{}`", name))
                            .with_primary_label(ty.location())
                            .with_secondary_label(
                                occ.get().location(),
                                format!("Previous definition of type `{}`", name),
                            ),
                    );
                }
            }
        };

        let mut try_add_interface =
            |interface: Arc<Interface>| match interfaces.entry(interface.name.clone()) {
                Entry::Vacant(v) => {
                    v.insert(interface);
                }
                Entry::Occupied(o) => diagnostics.borrow_mut().push(
                    Diagnostic::error()
                        .with_message(format!(
                            "Multiple definitions of interface `{}`",
                            interface.name
                        ))
                        .with_primary_label(interface.location)
                        .with_secondary_label(
                            o.get().location,
                            format!("Previous definition of interface `{}`", interface.name),
                        ),
                ),
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
                IpcFileItem::InterfaceDef(i) => try_add_interface(i.clone()),
            }
        }

        let mut diagnostics = diagnostics.take();

        let context = TypecheckContext {
            named_types,
            interfaces,
        };

        for (_, named_type) in context.named_types.iter() {
            if let Err(e) = named_type.typecheck(&context) {
                diagnostics.extend(e);
            }
        }

        for (_, interface) in context.interfaces.iter() {
            if let Err(e) = interface.typecheck(&context) {
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
