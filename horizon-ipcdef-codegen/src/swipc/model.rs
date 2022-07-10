use crate::swipc::diagnostics;
use crate::swipc::diagnostics::{DiagnosticExt, DiagnosticResultExt, Span};
use arcstr::ArcStr;
use codespan_reporting::diagnostic::Diagnostic;
use derivative::Derivative;
use itertools::Itertools;
use once_cell::sync::Lazy;
use regex::Regex;
use std::cell::RefCell;
use std::collections::btree_map::Entry;
use std::collections::BTreeMap;
use std::fmt::{Debug, Display, Formatter};
use std::sync::Arc;

pub type Namespace = Arc<Vec<ArcStr>>;

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct NamespacedIdent {
    namespace: Namespace,
    ident: ArcStr,
}

static IDENT_PART_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[a-zA-Z_]\w*$").unwrap());

impl NamespacedIdent {
    pub fn new(namespace: Arc<Vec<ArcStr>>, ident: ArcStr) -> Self {
        Self { namespace, ident }
    }

    pub fn from_parts(parts: Vec<ArcStr>) -> Self {
        assert!(!parts.is_empty());

        let mut it = parts.into_iter().peekable();

        let mut namespace = Vec::new();

        loop {
            let part = it.next().unwrap();

            if it.peek().is_some() {
                namespace.push(part);
            } else {
                return Self::new(Arc::new(namespace), part);
            }
        }
    }

    pub fn parse(s: &str) -> anyhow::Result<Self> {
        let mut res = Vec::new();

        for part in s.split("::") {
            if !IDENT_PART_REGEX.is_match(part) {
                return Err(anyhow::anyhow!("Some part of the identifier contained symbols that are not allowed or started with a number"));
            }

            res.push(ArcStr::from(part));
        }

        if res.is_empty() {
            return Err(anyhow::anyhow!("Empty identifiers are not allowed"));
        }

        Ok(Self::from_parts(res))
    }

    pub fn iter_namespaces(&self) -> impl Iterator<Item = &ArcStr> {
        self.namespace.iter()
    }

    pub fn namespace(&self) -> &Namespace {
        &self.namespace
    }

    pub fn ident(&self) -> &ArcStr {
        &self.ident
    }
}

impl Debug for NamespacedIdent {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for namespace in self.iter_namespaces() {
            write!(f, "{}::", namespace)?;
        }
        write!(f, "{}", self.ident())?;
        Ok(())
    }
}

impl Display for NamespacedIdent {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

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
        name: NamespacedIdent,
        #[derivative(PartialEq = "ignore")]
        reference_location: Span,
    },
}

impl NominalType {
    fn typecheck_resolve_with_depth(
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

                    t.referenced_type
                        .typecheck_resolve_with_depth(context, depth_limit - 1)
                        .with_context(reference_location, || {
                            format!("While resolving type named `{}`", name)
                        })?
                }
                TypeWithName::StructDef(s) => StructuralType::Struct(s),
                TypeWithName::EnumDef(e) => StructuralType::Enum(e),
                TypeWithName::BitflagsDef(b) => StructuralType::Bitflags(b),
            },
        })
    }

    pub fn typecheck_resolve(
        &self,
        context: &TypecheckContext,
    ) -> diagnostics::Result<StructuralType> {
        self.typecheck_resolve_with_depth(context, 16)
    }

    /// Resolve type in a codegen stage
    /// This is supposed to be infallible because typecheck should have caught all unresolved references
    pub fn codegen_resolve(&self, context: &CodegenContext) -> StructuralType {
        match self {
            &NominalType::Int(i) => StructuralType::Int(i),
            NominalType::Bool => StructuralType::Bool,
            NominalType::F32 => StructuralType::F32,
            &NominalType::Bytes { size, alignment } => StructuralType::Bytes { size, alignment },
            &NominalType::Unknown { size } => StructuralType::Unknown { size },
            NominalType::TypeName { name, .. } => context
                .resolved_type_names
                .get(name)
                .expect("Bug: unresolved reference slipped into codegen stage")
                .clone(),
        }
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
    InObject(NamespacedIdent, Span),
    /// sf::Out<sf::SharedPointer<T>>
    OutObject(Option<NamespacedIdent>, Span),

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
    pub name: NamespacedIdent,
    pub is_large_data: bool,
    pub preferred_transfer_mode: Option<BufferTransferMode>,
    pub fields: Vec<StructField>,
    #[derivative(PartialEq = "ignore")]
    pub location: Span,
}

impl Struct {
    pub fn try_new(
        name: NamespacedIdent,
        fields: Vec<StructField>,
        markers: Vec<StructMarker>,
        location: Span,
    ) -> diagnostics::Result<Self> {
        let is_large_data = markers.iter().any(|v| v == &StructMarker::LargeData);
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
    pub name: NamespacedIdent,
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
    pub name: NamespacedIdent,
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
    pub name: NamespacedIdent,
    /// Whether the code generated should be using domain objects or not
    pub is_domain: bool,
    pub sm_names: Vec<ArcStr>,
    pub commands: Vec<Command>,
    #[derivative(PartialEq = "ignore")]
    pub location: Span,
}

#[derive(Debug, Clone, Derivative)]
#[derivative(PartialEq)]
pub struct TypeAlias {
    pub name: NamespacedIdent,
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
    pub fn name(&self) -> &NamespacedIdent {
        match self {
            TypeWithName::TypeAlias(a) => &a.name,
            TypeWithName::StructDef(s) => &s.name,
            TypeWithName::EnumDef(e) => &e.name,
            TypeWithName::BitflagsDef(b) => &b.name,
        }
    }

    pub fn location(&self) -> Span {
        match self {
            TypeWithName::TypeAlias(a) => a.location,
            TypeWithName::StructDef(s) => s.location,
            TypeWithName::EnumDef(e) => e.location,
            TypeWithName::BitflagsDef(b) => b.location,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct TypecheckContext {
    pub named_types: BTreeMap<NamespacedIdent, TypeWithName>,
    pub interfaces: BTreeMap<NamespacedIdent, Arc<Interface>>,
}

impl TypecheckContext {
    pub fn resolve_type(
        &self,
        name: &NamespacedIdent,
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
        name: &NamespacedIdent,
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
pub struct CodegenContext {
    resolved_type_names: BTreeMap<NamespacedIdent, StructuralType>,
    resolved_interfaces: BTreeMap<NamespacedIdent, Arc<Interface>>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct IpcFile {
    items: Vec<IpcFileItem>,
    context: CodegenContext,
}

impl IpcFile {
    pub fn try_new(items: Vec<IpcFileItem>) -> diagnostics::Result<Self> {
        let mut named_types = BTreeMap::new();
        let mut interfaces = BTreeMap::new();

        let res = RefCell::new(Ok(()));

        let mut try_add_named_type = |ty: TypeWithName| {
            let name = ty.name().clone();
            let e = named_types.entry(name.clone());
            match e {
                Entry::Vacant(vac) => {
                    vac.insert(ty);
                }
                Entry::Occupied(occ) => {
                    res.borrow_mut().push(
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
                Entry::Occupied(o) => res.borrow_mut().push(
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

        let mut res = res.into_inner();

        let context = TypecheckContext {
            named_types,
            interfaces,
        };

        let mut resolved_type_names = BTreeMap::new();

        for (_, named_type) in context.named_types.iter() {
            if let Some(resolved) = res.extend_result(named_type.resolve_and_typecheck(&context)) {
                resolved_type_names.insert(named_type.name().clone(), resolved);
            }
        }

        for (_, interface) in context.interfaces.iter() {
            res.extend_result(interface.typecheck(&context));
        }

        let context = CodegenContext {
            resolved_interfaces: context.interfaces,
            resolved_type_names,
        };

        res.map(|_| Self { items, context })
    }

    pub fn iter_items(&self) -> impl Iterator<Item = &IpcFileItem> {
        self.items.iter()
    }

    pub fn context(&self) -> &CodegenContext {
        &self.context
    }
}
