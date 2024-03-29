use crate::swipc::codegen::types::make_nominal_type;
use crate::swipc::codegen::{import_in, make_ident, TokenStorage};
use crate::swipc::diagnostics::Span;
use crate::swipc::layout::FieldsLayoutItem;
use crate::swipc::model::{
    BufferExtraAttrs, BufferTransferMode, CodegenContext, Command, Direction, HandleTransferType,
    IntType, Interface, Namespace, NamespacedIdent, NominalType, Struct, StructField, Value,
};
use crate::swipc::util::PaddingHelper;
use arcstr::ArcStr;
use convert_case::{Case, Casing};
use genco::lang::rust::Tokens;
use genco::prelude::*;
use std::sync::Arc;

fn make_interface_reference(current_namespace: &Namespace, name: &NamespacedIdent) -> Tokens {
    import_in(current_namespace, name)
}

fn imp_handle_storage() -> Tokens {
    let imp = rust::import("horizon_ipc::handle_storage", "HandleStorage");

    quote!($imp)
}

fn imp_owned_handle() -> Tokens {
    let imp = rust::import("horizon_ipc::handle_storage", "OwnedHandle");

    quote!($imp)
}

fn imp_ref_handle() -> Tokens {
    let imp = rust::import("horizon_ipc::handle_storage", "RefHandle");

    quote!($imp)
}

fn imp_shared_handle() -> Tokens {
    let imp = rust::import("horizon_ipc::handle_storage", "SharedHandle");

    quote!($imp)
}

fn imp_pooled_handle() -> Tokens {
    let imp = rust::import("horizon_ipc::handle_storage", "PooledHandle");

    quote!($imp)
}

fn imp_error_code() -> Tokens {
    let imp = rust::import("horizon_error", "ErrorCode");

    quote!($imp)
}

fn imp_result() -> Tokens {
    let imp = rust::import("horizon_error", "Result");

    quote!($imp)
}

fn imp_raw_handle() -> Tokens {
    let imp = rust::import("horizon_ipc", "RawHandle");

    quote!($imp)
}

fn imp_maybe_uninit() -> Tokens {
    let imp = rust::import("core::mem", "MaybeUninit");

    quote!($imp)
}

fn imp_hipc_header() -> Tokens {
    let imp = rust::import("horizon_ipc::raw::hipc", "HipcHeader");

    quote!($imp)
}

fn imp_hipc_special_header() -> Tokens {
    let imp = rust::import("horizon_ipc::raw::hipc", "HipcSpecialHeader");

    quote!($imp)
}

fn imp_in_pointer_desc() -> Tokens {
    let imp = rust::import("horizon_ipc::raw::hipc", "HipcInPointerBufferDescriptor");

    quote!($imp)
}

fn imp_out_pointer_desc() -> Tokens {
    let imp = rust::import("horizon_ipc::raw::hipc", "HipcOutPointerBufferDescriptor");

    quote!($imp)
}

fn imp_map_alias_desc() -> Tokens {
    let imp = rust::import("horizon_ipc::raw::hipc", "HipcMapAliasBufferDescriptor");

    quote!($imp)
}

fn imp_cmif_in_header() -> Tokens {
    let imp = rust::import("horizon_ipc::raw::cmif", "CmifInHeader");

    quote!($imp)
}

fn imp_cmif_out_header() -> Tokens {
    let imp = rust::import("horizon_ipc::raw::cmif", "CmifOutHeader");

    quote!($imp)
}

fn imp_get_ipc_buffer_for() -> Tokens {
    let imp = rust::import("horizon_ipc::buffer", "get_ipc_buffer_for");

    quote!($imp)
}

fn imp_get_ipc_buffer_ptr() -> Tokens {
    let imp = rust::import("horizon_ipc::buffer", "get_ipc_buffer_ptr");

    quote!($imp)
}

fn imp_command_type() -> Tokens {
    let imp = rust::import("horizon_ipc::cmif", "CommandType");

    quote!($imp)
}

#[derive(Clone)]
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

#[derive(Clone)]
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
    Owned,
    Interface(NamespacedIdent),
}

struct CommandInterfaceInfo {
    args: Vec<(ArcStr, Tokens)>,
    results: Vec<(ArcStr, Tokens)>,
    uninit_vars: Vec<(ArcStr, Tokens)>,
}

struct CommandWireFormatInfo {
    is_domain: bool,
    command_id: u32,
    buffers: Vec<Buffer>,
    raw_data_in: Vec<RawDataIn>,
    raw_data_out: Vec<RawDataOut>,
    handles_in: Vec<HandleIn>,
    handles_out: Vec<HandleOut>,
    should_pass_pid: bool,
}

impl CommandWireFormatInfo {
    pub fn has_in_special_header(&self) -> bool {
        self.should_pass_pid || !self.handles_in.is_empty()
    }

    pub fn has_out_special_header(&self) -> bool {
        !self.handles_out.is_empty()
    }

    fn get_buffers(&self, mut filter: impl FnMut(&Buffer) -> bool) -> Vec<Buffer> {
        self.buffers
            .iter()
            .filter(move |b| filter(b))
            .cloned()
            .collect::<Vec<_>>()
    }

    pub fn in_pointer_buffers(&self) -> Vec<Buffer> {
        self.get_buffers(|b| {
            b.direction == Direction::In && b.transfer_mode == BufferTransferMode::Pointer
        })
    }

    pub fn out_pointer_buffers(&self) -> Vec<Buffer> {
        self.get_buffers(|b| {
            b.direction == Direction::Out && b.transfer_mode == BufferTransferMode::Pointer
        })
    }

    pub fn out_pointer_sizes_count(&self) -> usize {
        self.out_pointer_buffers()
            .iter()
            .filter(|b| !b.fixed_size)
            .count()
    }

    pub fn in_map_alias_buffers(&self) -> Vec<Buffer> {
        self.get_buffers(|b| {
            b.direction == Direction::In && b.transfer_mode == BufferTransferMode::MapAlias
        })
    }

    pub fn out_map_alias_buffers(&self) -> Vec<Buffer> {
        self.get_buffers(|b| {
            b.direction == Direction::Out && b.transfer_mode == BufferTransferMode::MapAlias
        })
    }

    pub fn in_copy_handles(&self) -> usize {
        self.handles_in
            .iter()
            .filter(|h| h.transfer_type == HandleTransferType::Copy)
            .count()
    }

    pub fn in_move_handles(&self) -> usize {
        self.handles_in
            .iter()
            .filter(|h| h.transfer_type == HandleTransferType::Move)
            .count()
    }

    pub fn out_copy_handles(&self) -> usize {
        self.handles_out
            .iter()
            .filter(|h| h.transfer_type == HandleTransferType::Copy)
            .count()
    }

    pub fn out_move_handles(&self) -> usize {
        self.handles_out
            .iter()
            .filter(|h| h.transfer_type == HandleTransferType::Move)
            .count()
    }

    pub fn in_raw_data_struct(&self) -> Struct {
        raw_data_struct(
            self.raw_data_in
                .iter()
                .map(|d| (d.name.clone(), d.ty.clone())),
        )
    }

    pub fn out_raw_data_struct(&self) -> Struct {
        raw_data_struct(
            self.raw_data_out
                .iter()
                .map(|d| (d.name.clone(), d.ty.clone())),
        )
    }
}

fn collect_command_info(
    namespace: &Namespace,
    ctx: &CodegenContext,
    is_domain: bool,
    command: &Command,
) -> (CommandInterfaceInfo, CommandWireFormatInfo) {
    let mut args = Vec::new();
    let mut results = Vec::new();
    let mut uninit_vars = Vec::new();

    let mut buffers = Vec::new();
    let mut raw_data_in = Vec::new();
    let mut raw_data_out = Vec::new();
    let mut handles_in = Vec::new();
    let mut handles_out = Vec::new();

    let mut should_pass_pid = false;

    for (name, arg) in command.arguments.iter() {
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
                        source: BufferSource::TypedUninitVariable(name.clone()),
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
                        transfer_type: HandleTransferType::Move,
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
                        $(imp_raw_handle())
                    },
                ));
            }
            &Value::OutHandle(transfer_type) => {
                handles_out.push(HandleOut {
                    name: name.clone(),
                    transfer_type,
                    transform: HandleTransformType::Owned,
                });

                results.push((
                    name,
                    quote! {
                        $(imp_owned_handle())
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

    (
        CommandInterfaceInfo {
            args,
            results,
            uninit_vars,
        },
        CommandWireFormatInfo {
            is_domain,
            command_id: command.id,
            buffers,
            raw_data_in,
            raw_data_out,
            handles_in,
            handles_out,
            should_pass_pid,
        },
    )
}

fn raw_data_struct(items: impl Iterator<Item = (ArcStr, NominalType)>) -> Struct {
    let s = Struct::try_new(
        NamespacedIdent::new(Arc::new(Vec::new()), arcstr::literal!("RawData")),
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

    s
}

fn make_raw_data_struct(
    namespace: &Namespace,
    ctx: &CodegenContext,
    direction: Direction,
    s: &Struct,
) -> Tokens {
    let name = NamespacedIdent::new(namespace.clone(), ArcStr::from(format!("{:?}", direction)));

    let size = s.layout(ctx).size();

    let name = name.ident().as_str();

    let mut padding_helper = PaddingHelper::new();

    quote! {
        #[repr(C, packed)]
        struct $name {
            $(for f in s.fields_layout(ctx).items {
                $(match f {
                    FieldsLayoutItem::Field(_, i) => {
                        pub $(make_ident(&s.fields[i].name)):
                            $(make_nominal_type(namespace, &s.fields[i].ty)),
                    }
                    FieldsLayoutItem::Padding(size) => {
                        pub $(padding_helper.next_padding_name()): [u8; $size],
                    }
                })
            })
        }

        let _ = ::core::mem::transmute::<$name, [u8; $size]>;
    }
}

fn make_raw_data_in_type(
    namespace: &Namespace,
    _ctx: &CodegenContext,
    data: &[RawDataIn],
) -> Tokens {
    if data.is_empty() {
        (quote! {
            ()
        } as Tokens)
    } else if let [data] = data {
        quote! {
            $(match data.source {
                RawDataInSource::PidPlaceholder => u64,
                RawDataInSource::Local =>
                    $(make_nominal_type(namespace, &data.ty)),
            })
        }
    } else {
        quote! {
            In
        }
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
        let s = raw_data_struct(data.iter().map(|d| (d.name.clone(), d.ty.clone())));
        let mut padding_helper = PaddingHelper::new();

        quote! {
            $(make_raw_data_struct(
                namespace,
                ctx,
                Direction::In,
                &s
            ))

            let data_in: In = In {
                $(for data in data {
                    $(match data.source {
                        RawDataInSource::PidPlaceholder =>
                            $(data.name.as_str()): 0,

                        RawDataInSource::Local =>
                            $(data.name.as_str()),
                    }),
                })

                $(for _ in s.paddings(ctx) {
                    $(padding_helper.next_padding_name()): Default::default(),
                })
            };
        }
    }
}

fn make_raw_data_out_type(
    namespace: &Namespace,
    ctx: &CodegenContext,
    data: &[RawDataOut],
) -> Tokens {
    if data.is_empty() {
        (quote! {
            ()
        } as Tokens)
    } else if let [data] = data {
        quote! {
            $(make_nominal_type(namespace, &data.ty))
        }
    } else {
        quote! {
            Out
        }
    }
}

fn make_raw_data_out_struct(
    namespace: &Namespace,
    ctx: &CodegenContext,
    data: &[RawDataOut],
) -> Tokens {
    if data.len() <= 1 {
        (quote! {} as Tokens)
    } else {
        let s = raw_data_struct(data.iter().map(|d| (d.name.clone(), d.ty.clone())));

        quote! {
            $(make_raw_data_struct(
                namespace,
                ctx,
                Direction::Out,
                &s
            ))
        }
    }
}

#[derive(Debug)]
struct RequestSizes {
    cmif_header_offset: usize,
    data_size: usize,
    request_size: usize,
}

fn request_sizes(ctx: &CodegenContext, w_info: &CommandWireFormatInfo) -> RequestSizes {
    let &CommandWireFormatInfo {
        should_pass_pid,
        ref handles_in,
        ..
    } = w_info;

    let in_pointer_buffers = w_info.in_pointer_buffers();
    let out_pointer_buffers = w_info.out_pointer_buffers();
    let in_map_aliases = w_info.in_map_alias_buffers();
    let out_map_aliases = w_info.out_map_alias_buffers();
    let out_pointer_sizes_count = w_info.out_pointer_sizes_count();

    let cmif_header_offset = 8 + // HIPC header
        if w_info.has_in_special_header() {
            4 + // special header
                (should_pass_pid as usize) * 8 +
                handles_in.len() * 4
        } else { 0 } +
        in_pointer_buffers.len() * 8 + // descriptors
        in_map_aliases.len() * 12 + // descriptors
        out_map_aliases.len() * 12; // descriptors

    let raw_data_size = w_info.in_raw_data_struct().layout(ctx).size();

    let data_size = 16 + // padding
        16 + // CMIF header
        raw_data_size as usize +
        ((4 - raw_data_size % 4) % 4) as usize + // pad raw data to word size (4 bytes)
        out_pointer_sizes_count * 2 + // OutPointer lengths as a u16 array
        if out_pointer_sizes_count % 2 != 0 { // padding for OutPointer length array
            2
        } else  {
            0
        };

    let request_size = cmif_header_offset + data_size + out_pointer_buffers.len() * 8; // descriptors

    RequestSizes {
        cmif_header_offset,
        data_size,
        request_size,
    }
}

struct ResponseSizes {
    cmif_header_offset: usize,
    data_size: usize,
    response_size: usize,
    cmif_alternative_result_offset: usize,
}

fn response_sizes(ctx: &CodegenContext, w_info: &CommandWireFormatInfo) -> ResponseSizes {
    let &CommandWireFormatInfo {
        should_pass_pid,
        ref handles_out,
        ..
    } = w_info;

    let out_pointer_buffers = w_info.out_pointer_buffers();

    let cmif_header_offset = 8 + // HIPC header
        if w_info.has_out_special_header() {
            4 + // special header
                handles_out.len() * 4
        } else { 0 } +
        out_pointer_buffers.len() * 8; // descriptors (would be in_pointer)

    // offset to result in case no handles are sent
    let cmif_alternative_header_offset = 8 + // HIPC header
        out_pointer_buffers.len() * 8;

    // align up to 16 bytes
    let cmif_alternative_header_offset =
        cmif_alternative_header_offset + (16 - cmif_alternative_header_offset % 16) % 16;
    let cmif_alternative_result_offset = cmif_alternative_header_offset + 8;

    let raw_data_size = w_info.out_raw_data_struct().layout(ctx).size();

    let data_size = 16 + // padding
        16 + // CMIF header
        raw_data_size as usize +
        ((4 - raw_data_size % 4) % 4) as usize; // pad raw data to word size (4 bytes)

    let response_size = cmif_header_offset + data_size;

    ResponseSizes {
        cmif_header_offset,
        data_size,
        response_size,
        cmif_alternative_result_offset,
    }
}

fn make_request_struct(
    namespace: &Namespace,
    ctx: &CodegenContext,
    w_info: &CommandWireFormatInfo,
) -> Tokens {
    let &CommandWireFormatInfo {
        should_pass_pid,
        ref handles_in,
        ..
    } = w_info;

    let in_pointer_buffers = w_info.in_pointer_buffers();
    let out_pointer_buffers = w_info.out_pointer_buffers();
    let in_map_aliases = w_info.in_map_alias_buffers();
    let out_map_aliases = w_info.out_map_alias_buffers();

    if w_info.is_domain {
        todo!("Domain codegen")
    }

    let out_pointer_sizes_count = w_info.out_pointer_sizes_count();

    let RequestSizes {
        cmif_header_offset,
        request_size,
        ..
    } = request_sizes(ctx, w_info);

    if request_size > 0x100 {
        panic!("Request struct is too large, would not fit into the IPC command buffer")
    }

    // use the offset to calculate cmif padding size
    let pre_cmif_padding = (16 - cmif_header_offset % 16) % 16;

    let raw_data_size = w_info.in_raw_data_struct().layout(ctx).size();

    let raw_data_word_padding = (4 - raw_data_size % 4) % 4;

    let r: Tokens = quote! {
        #[repr(packed)]
        struct Request {
            hipc: $(imp_hipc_header()),
            $(if w_info.has_in_special_header() {
                special_header: $(imp_hipc_special_header()),
                $(if should_pass_pid => pid_placeholder: u64,)
                $(for h in handles_in {
                    $(format!("handle_{}", h.name)): $(imp_raw_handle()),
                })
            })
            $(for (i, _) in in_pointer_buffers.iter().enumerate() {
                $(format!("in_pointer_desc_{}", i)): $(imp_in_pointer_desc()),
            })
            $(for (i, _) in in_map_aliases.iter().enumerate() {
                $(format!("in_map_alias_desc_{}", i)): $(imp_map_alias_desc()),
            })
            $(for (i, _) in out_map_aliases.iter().enumerate() {
                $(format!("out_map_alias_desc_{}", i)): $(imp_map_alias_desc()),
            })

            pre_padding: [u8; $pre_cmif_padding],
            cmif: $(imp_cmif_in_header()),
            raw_data: $(make_raw_data_in_type(namespace, ctx, &w_info.raw_data_in)),
            raw_data_word_padding: [u8; $raw_data_word_padding],
            post_padding: [u8; $(16 - pre_cmif_padding)],
            $(for (i, b) in out_pointer_buffers.iter().enumerate() {
                $(if !b.fixed_size {
                    $(format!("out_pointer_size_{}", i)): u16,
                })
            })
            $(if out_pointer_sizes_count % 2 != 0 {
                out_pointer_size_padding: u16,
            })


            $(for (i, _) in out_pointer_buffers.iter().enumerate() {
                $(format!("out_pointer_desc_{}", i)): $(imp_out_pointer_desc()),
            })
        }

        _comment_!("Compiler time request size check");
        let _ = ::core::mem::transmute::<Request, [u8; $(request_size)]>;
    };

    r
}

fn make_response_struct(
    namespace: &Namespace,
    ctx: &CodegenContext,
    w_info: &CommandWireFormatInfo,
) -> Tokens {
    let &CommandWireFormatInfo {
        ref handles_out, ..
    } = w_info;

    let out_pointer_buffers = w_info.out_pointer_buffers();

    if w_info.is_domain {
        todo!("Domain codegen")
    }

    let ResponseSizes {
        cmif_header_offset,
        data_size,
        response_size,
        ..
    } = response_sizes(ctx, w_info);

    if response_size > 0x100 {
        panic!("Request struct is too large, would not fit into the IPC command buffer")
    }

    // use the offset to calculate cmif padding size
    let pre_cmif_padding = (16 - (cmif_header_offset) % 16) % 16;

    let raw_data_size = w_info.out_raw_data_struct().layout(ctx).size();

    let raw_data_word_padding = (4 - (raw_data_size % 4)) % 4;

    let r: Tokens = quote! {
        #[repr(packed)]
        struct Response {
            hipc: $(imp_hipc_header()),
            $(if w_info.has_out_special_header() {
                special_header: $(imp_hipc_special_header()),
                $(for h in handles_out {
                    $(format!("handle_{}", h.name)): $(imp_raw_handle()),
                })
            })
            $(for (i, _) in out_pointer_buffers.iter().enumerate() {
                $(format!("in_pointer_desc_{}", i)): $(imp_in_pointer_desc()),
            })

            pre_padding: [u8; $pre_cmif_padding],
            cmif: $(imp_cmif_out_header()),
            raw_data: $(make_raw_data_out_type(namespace, ctx, &w_info.raw_data_out)),
            raw_data_word_padding: [u8; $raw_data_word_padding],
            post_padding: [u8; $(16 - pre_cmif_padding)],
        }

        _comment_!("Compiler time request size check");
        let _ = ::core::mem::transmute::<Response, [u8; $response_size]>;
    };

    r
}

fn make_response_pattern(ctx: &CodegenContext, w_info: &CommandWireFormatInfo) -> Tokens {
    let CommandWireFormatInfo { handles_out, .. } = w_info;

    let data = w_info.raw_data_out.as_slice();

    let raw_data_pattern: Tokens = if data.is_empty() {
        (quote! {
            ()
        } as Tokens)
    } else if let [data] = data {
        quote! {
            $(data.name.as_str())
        }
    } else {
        quote! {
            Out {
                $(for data in data {
                    $(data.name.as_str()),
                })
            }
        }
    };

    (quote! {
        Response {
            hipc,
            $(if w_info.has_out_special_header() {
                special_header,
                $(for h in handles_out {
                    $(format!("handle_{}", h.name)): $(h.name.as_str()),
                })
            })
            // I don't think we care about those?
            // $(for (i, _) in out_pointer_buffers.iter().enumerate() {
            //     $(format!("in_pointer_desc_{}", i)),
            // })

            cmif,
            raw_data: $raw_data_pattern,

            ..
        }
    } as Tokens)
}

fn make_error_return(ctx: &CodegenContext, w_info: &CommandWireFormatInfo) -> Tokens {
    // well, this is embarrassing
    // when returning an error, the server will not send us handles that it usually will,
    //  so it would change the layout and potentially shift the CMIF header,
    //  rendering the error code usually read incorrect
    // This is why a clever solution is used: if there is no special header where it should be,
    //  we assume that an error happened and read the error code at fixed offset to return it early

    let ResponseSizes {
        cmif_alternative_result_offset,
        ..
    } = response_sizes(ctx, w_info);

    (quote! {
        $(if !w_info.has_out_special_header() {
            if cmif.result.is_failure() {
                return Err(cmif.result)
            }
        } else {
            if hipc.has_special_header() != 0 {
                if cmif.result.is_failure() {
                    return Err(cmif.result)
                }
            } else {
                return Err(
                    unsafe {
                        ::core::ptr::read(
                            ipc_buffer_ptr.offset($cmif_alternative_result_offset)
                                as *const $(imp_error_code())
                        )
                    }
                )
            }
        })
    } as Tokens)
}

fn make_check_response(ctx: &CodegenContext, w_info: &CommandWireFormatInfo) -> Tokens {
    let ResponseSizes {
        cmif_header_offset,
        data_size,
        response_size,
        ..
    } = response_sizes(ctx, w_info);

    let num_in_pointers = w_info.out_pointer_buffers().len();

    let has_special_header = w_info.has_out_special_header();

    let num_copy_handles = w_info.out_copy_handles();
    let num_move_handles = w_info.out_move_handles();

    (quote! {
        debug_assert_eq!(hipc.num_in_pointers(), $num_in_pointers);
        debug_assert_eq!(hipc.num_in_map_aliases(), 0);
        debug_assert_eq!(hipc.num_out_map_aliases(), 0);
        debug_assert_eq!(hipc.num_inout_map_aliases(), 0);

        // yuzu currently calculates this incorrectly
        // debug_assert_eq!(hipc.num_data_words(), $data_size);

        debug_assert_eq!(hipc.out_pointer_mode(), 0);
        debug_assert_eq!(hipc.has_special_header(), $(has_special_header as u32));

        $(if has_special_header {
            debug_assert_eq!(special_header.send_pid(), 0);
            debug_assert_eq!(special_header.num_copy_handles(), $num_copy_handles);
            debug_assert_eq!(special_header.num_move_handles(), $num_move_handles);
        })

        debug_assert_eq!(cmif.magic, $(imp_cmif_out_header())::MAGIC);
    } as Tokens)
}

fn make_buffer_size(buffer: &Buffer) -> Tokens {
    (match &buffer.source {
        BufferSource::TypedUninitVariable(name) => {
            quote! {
                ::core::mem::size_of_val(&$(name.as_str())) as u16
            }
        }

        BufferSource::TypedReference(name)
        | BufferSource::TypedSlice(name)
        | BufferSource::ByteSlice(name) => {
            quote! {
                ::core::mem::size_of_val($(name.as_str())) as u16
            }
        }
    }) as Tokens
}

enum DescriptorType {
    MapAlias,
    InPointer,
    OutPointer,
}

fn make_buffer_desc(ty: DescriptorType, index: usize, buffer: &Buffer) -> Tokens {
    let (addr, size) = match &buffer.source {
        BufferSource::ByteSlice(name) | BufferSource::TypedSlice(name) => (
            quote! {
                $(name.as_str()).as_ptr() as usize
            },
            quote! {
                ::core::mem::size_of_val($(name.as_str()))
            },
        )
            as (Tokens, Tokens),
        BufferSource::TypedUninitVariable(name) => (
            quote! {
                $(name.as_str()).as_ptr() as usize
            },
            quote! {
                ::core::mem::size_of_val(&$(name.as_str()))
            },
        ),
        BufferSource::TypedReference(name) => (
            quote! {
                $(name.as_str()) as *const _ as usize
            },
            quote! {
                ::core::mem::size_of_val($(name.as_str()))
            },
        ),
    };

    let alias_desc = rust::import("horizon_ipc::raw::hipc", "HipcMapAliasBufferDescriptor");
    let ptr_in_desc = rust::import("horizon_ipc::raw::hipc", "HipcInPointerBufferDescriptor");
    let ptr_out_desc = rust::import("horizon_ipc::raw::hipc", "HipcOutPointerBufferDescriptor");

    let alias_buffer_mode = rust::import("horizon_ipc::hipc", "MapAliasBufferMode");

    let extra_attrs = quote! {
        $(match buffer.extra_attrs {
            BufferExtraAttrs::None => $alias_buffer_mode::Normal,
            BufferExtraAttrs::AllowNonSecure => $alias_buffer_mode::NonSecure,
            BufferExtraAttrs::AllowNonDevice => $alias_buffer_mode::NonDevice,
        })
    };

    (quote! {
        $(match ty {
            DescriptorType::MapAlias => {
                $alias_desc::new(
                    $extra_attrs,
                    $addr,
                    $size
                )
            }
            DescriptorType::InPointer => {
                $(if buffer.transfer_mode == BufferTransferMode::AutoSelect {
                    // TODO: use pointer transfer mode if enough space in pointer buffer
                    // need to decide that at runtime though
                    $ptr_in_desc::new($index, 0, 0)
                } else {
                    $ptr_in_desc::new($index, $addr, $size)
                })
            }
            DescriptorType::OutPointer => {
                $(if buffer.transfer_mode == BufferTransferMode::AutoSelect {
                    // TODO: use pointer transfer mode if enough space in pointer buffer
                    // need to decide that at runtime though
                    $ptr_out_desc::new(0, 0)
                } else {
                    $ptr_out_desc::new($addr, $size)
                })
            }
        })
    } as Tokens)
}

fn make_request(ctx: &CodegenContext, w_info: &CommandWireFormatInfo) -> Tokens {
    let &CommandWireFormatInfo {
        should_pass_pid,
        command_id,
        ref handles_in,
        ..
    } = w_info;

    let in_pointer_buffers = w_info.in_pointer_buffers();
    let out_pointer_buffers = w_info.out_pointer_buffers();
    let in_map_aliases = w_info.in_map_alias_buffers();
    let out_map_aliases = w_info.out_map_alias_buffers();

    let out_pointer_sizes_count = w_info.out_pointer_sizes_count();

    // switchbrew:
    // > If it has value 0, the C descriptor functionality is disabled.
    // > If it has value 1, there is an "inlined" C buffer after the raw data.
    // >  Received data is copied to ROUND_UP(cmdbuf+raw_size+index, 16)
    // > If it has value 2, there is a single C descriptor.
    // > Otherwise it has (flag-2) C descriptors.
    // >  In this case, index picks which C descriptor to copy received data to
    // >  [instead of picking the offset into the buffer].
    //
    // we do not use neither the single C (OutPointer) descriptor (because it's for servers)
    //  nor an "inlined" buffer after the raw data
    // so, it's either nothing or 2+N

    let sizes = request_sizes(ctx, w_info);
    assert_eq!(sizes.data_size % 4, 0, "data_size should be multiple of 4");

    let out_pointer_mode = if out_pointer_buffers.is_empty() {
        0
    } else {
        2 + out_pointer_buffers.len()
    };

    let r: Tokens = quote! {
        Request {
            hipc: $(imp_hipc_header())::new(
                $(imp_command_type())::Request,
                $(in_pointer_buffers.len()),
                $(in_map_aliases.len()),
                $(out_map_aliases.len()),
                0, // num_inout_map_aliases
                $(sizes.data_size / 4), // num_data_words
                $(out_pointer_mode),
                0, // recv_list_offset
                $(if w_info.has_in_special_header() {
                    true
                } else {
                    false
                }),
            ),
            $(if w_info.has_in_special_header() {
                special_header: $(imp_hipc_special_header())::new(
                    $(if should_pass_pid {
                        true
                    } else {
                        false
                    }),
                    $(w_info.in_copy_handles()),
                    $(w_info.in_move_handles()),
                ),
                $(if should_pass_pid => pid_placeholder: 0,)
                $(for h in handles_in {
                    $(format!("handle_{}", h.name)): $(h.name.as_str()),
                })
            })
            $(for (i, b) in in_pointer_buffers.iter().enumerate() {
                $(format!("in_pointer_desc_{}", i)):
                    $(make_buffer_desc(DescriptorType::InPointer, i, b)),
            })
            $(for (i, b) in in_map_aliases.iter().enumerate() {
                $(format!("in_map_alias_desc_{}", i)):
                    $(make_buffer_desc(DescriptorType::MapAlias, i, b)),
            })
            $(for (i, b) in out_map_aliases.iter().enumerate() {
                $(format!("out_map_alias_desc_{}", i)):
                    $(make_buffer_desc(DescriptorType::MapAlias, i, b)),
            })

            pre_padding: Default::default(),
            cmif: $(imp_cmif_in_header()) {
                magic: $(imp_cmif_in_header())::MAGIC,
                version: 1,
                command_id: $command_id,
                token: 0,
            },
            raw_data: data_in,
            raw_data_word_padding: Default::default(),
            post_padding: Default::default(),

            $(for (i, b) in out_pointer_buffers.iter().enumerate() {
                $(if !b.fixed_size {
                    $(format!("out_pointer_size_{}", i)): $(make_buffer_size(b)),
                })
            })
            $(if out_pointer_sizes_count % 2 != 0 {
                out_pointer_size_padding: 0,
            })

            $(for (i, b) in out_pointer_buffers.iter().enumerate() {
                $(format!("out_pointer_desc_{}", i)):
                    $(make_buffer_desc(DescriptorType::OutPointer, i, b)),
            })
        }
    };

    r
}

fn make_command_body(
    namespace: &Namespace,
    ctx: &CodegenContext,
    interface: &Interface,
    command: &Command,
    i_info: &CommandInterfaceInfo,
    w_info: &CommandWireFormatInfo,
) -> Tokens {
    let fq_command_name = format!(
        "{}::{}::{}",
        namespace.join("::"),
        interface.name.ident(),
        command.name
    );
    let fq_command_name = &fq_command_name;

    let CommandInterfaceInfo {
        uninit_vars,
        results,
        ..
    } = i_info;
    let CommandWireFormatInfo {
        is_domain: _,
        command_id: _,
        buffers: _,
        raw_data_in,
        raw_data_out,
        handles_in: _,
        handles_out,
        should_pass_pid: _,
    } = w_info;

    let r: Tokens = quote! {
        // defines a data_in variable
        $(make_raw_data_in(namespace, ctx, &raw_data_in))
        $(make_raw_data_out_struct(namespace, ctx, &raw_data_out))

        $(make_request_struct(namespace, ctx, w_info))
        $(make_response_struct(namespace, ctx, w_info))

        $(for (name, ty) in uninit_vars {
            let $(name.as_str()) = $(imp_maybe_uninit())::<$ty>::uninit();
        })

        let ipc_buffer_ptr = unsafe {
            $(imp_get_ipc_buffer_ptr())()
        };

        // SAFETY: The pointer should be valid
        unsafe {
            ::core::ptr::write(
                ipc_buffer_ptr as *mut _,
                $(make_request(ctx, w_info))
            )
        };

        {
            let handle = self.handle.get();
            crate::pre_ipc_hook($(quoted(fq_command_name)), *handle);
            horizon_svc::send_sync_request(*handle)?;
            crate::post_ipc_hook($(quoted(fq_command_name)), *handle);
        }

        // SAFETY: The pointer should be valid
        let $(make_response_pattern(ctx, w_info))
            = unsafe {
                ::core::ptr::read(
                ipc_buffer_ptr as *const _,
                )
            };

        $(make_error_return(ctx, w_info))
        $(make_check_response(ctx, w_info))

        $(for (name, _) in uninit_vars {
            let $(name.as_str()) = unsafe { $(name.as_str()).assume_init() };
        })

        $(for h in handles_out {
            $(match &h.transform {
                HandleTransformType::Owned => {
                    let $(h.name.as_str()) = $(imp_owned_handle())::new($(h.name.as_str()));
                },
                HandleTransformType::Interface(interface) => {
                    let $(h.name.as_str()) =
                        $(make_interface_reference(namespace, interface)) {
                            handle: $(imp_owned_handle())::new($(h.name.as_str()))
                        };
                }
            })
        })

        Ok(
            $(if results.is_empty() {
                ()
            } else {
                $(if let [result] = &results[..] {
                    $(result.0.as_str())
                } else {
                    (
                        $(for result in results join (,) {
                            $(result.0.as_str())
                        })
                    )
                })
            })
        )
    };

    r
}

fn make_command(
    namespace: &Namespace,
    ctx: &CodegenContext,
    interface: &Interface,
    command: &Command,
    is_domain: bool,
) -> Tokens {
    let (i_info, w_info) = collect_command_info(namespace, ctx, is_domain, command);

    let return_type = if let [(_, res)] = i_info.results.as_slice() {
        quote!($res) as Tokens
    } else {
        // TODO: doing this we lose names. This is not __that__ bad, but kinda meh...
        quote! {
            (
                $(for (_, ty) in i_info.results.iter() join (,) => $ty)
            )
        }
    };

    // we expect command names in PascalCase, but convert them to snake_case when converting to rust
    let name = command.name.to_case(Case::Snake);
    quote! {
        pub fn $name(
            &self,
            $(for (name, ty) in &i_info.args join (,) => $(name.as_str()): $ty)
        ) -> $(imp_result())<$return_type> {
            $(make_command_body(namespace, ctx, interface, command, &i_info, &w_info))
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

    tok.push(
        namespace.clone(),
        quote! {
            pub struct $name<S: $(imp_handle_storage()) = $(imp_owned_handle())> {
                // the generated interface object owns the session handle!
                pub(crate) handle: S,
            }

            impl<S: $(imp_handle_storage())> $name<S> {
                pub fn new(handle: S) -> Self {
                    Self { handle }
                }

                pub fn into_inner(self) -> S {
                    self.handle
                }

                $(for command in i.commands.iter() join (_blank_!();) {
                    $(make_command(namespace, ctx, i, command, i.is_domain))
                })
            }

            impl $name<$(imp_owned_handle())> {
                pub fn as_ref(&self) -> $name<$(imp_ref_handle())<'_>> {
                    $name {
                        handle: self.handle.as_ref()
                    }
                }
                pub fn into_shared(self) -> $name<$(imp_shared_handle())> {
                    $name {
                        handle: $(imp_shared_handle())::new(self.handle.leak())
                    }
                }
            }

            impl ::core::fmt::Debug for $name {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    write!(f, $[str]($[const](name)$[const]("({})")), self.handle)
                }
            }
            _blank_!();
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

    #[ignore] // TODO: update when the codegen results for commands will be more or less stable
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
                #![allow(unreachable_code, unused_variables, non_upper_case_globals, clippy::all)]
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
