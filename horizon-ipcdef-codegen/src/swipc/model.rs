use arcstr::ArcStr;
use std::collections::BTreeMap;
use std::sync::Arc;

/// Types are those things that have a known byte representations
/// Usually they will be sent as-is via the wire (put into the payload), but sometimes (sf::LargeData) they would be sent as buffers instead (TODO)
///
/// They can be included in structures
#[derive(Debug, PartialEq, Clone)]
pub enum Type {
    Int(IntType),
    Bool,
    F32,
    Bytes { size: u64, alignment: u64 },
    Unknown { size: Option<u64> },
    Struct(Struct),
    Enum(Enum),
    Bitflags(Bitflags),
    Typedef(ArcStr),
}

#[derive(Debug, PartialEq, Clone)]
pub enum HandleTransferType {
    Move,
    Copy,
}

#[derive(Debug, PartialEq, Clone)]
pub enum HandleType {
    Session,
    Port,
    // TODO
}

#[derive(Debug, PartialEq, Clone)]
pub struct BufferType {
    // TODO
}

impl BufferType {
    pub fn try_from_id(_id: u64) -> anyhow::Result<Self> {
        Ok(Self {})
    }
}

/// Everything that can be sent or received using IPC
/// Includes stuff like PID descriptors, objects and handles
///
/// They can't be included in structures
///
/// TODO: bikeshed on the name
#[derive(Debug, PartialEq, Clone)]
pub enum Value {
    LiteralValue(Type),
    Pid,
    Handle {
        transfer_type: HandleTransferType,
        ty: Option<HandleType>,
    },
    /// An untyped buffer
    Buffer {
        // TODO: is it strong enough?
        transfer_type: BufferType,
        size: Option<u64>,
    },
    /// Equivalent to a buffer of the given data_type and transfer_type with a variable size.
    Array {
        ty: Type,
        transfer_type: BufferType,
    },
    Object {
        interface_name: Option<ArcStr>,
    },
    // TODO
}

#[derive(Debug, PartialEq, Clone)]
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

#[derive(Debug, PartialEq, Clone)]
pub struct Struct {
    // TODO donbt forger: we want to allow for different markers like sf::LargeData et al
    pub fields: Vec<(ArcStr, Type)>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Enum {
    pub base_type: IntType,
    pub arms: Vec<(ArcStr, u64)>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Bitflags {
    pub base_type: IntType,
    pub arms: Vec<(ArcStr, u64)>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Command {
    // TODO: do we want to support multiple versions & version requirements at all?
    pub id: u32,
    pub name: ArcStr,
    pub inputs: Vec<(Option<ArcStr>, Arc<Value>)>,
    pub outputs: Vec<(Option<ArcStr>, Arc<Value>)>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Interface {
    pub name: ArcStr,
    pub sm_names: Vec<ArcStr>,
    pub commands: Vec<Command>,
}

// pub enum IpcFileItem {
//     Typedef(ArcStr, Type),
//     Interface(Interface),
// }

#[derive(Debug, PartialEq, Clone)]
pub struct IpcFile {
    pub typedefs: BTreeMap<ArcStr, Type>,
    pub interfaces: Vec<Interface>,
}
