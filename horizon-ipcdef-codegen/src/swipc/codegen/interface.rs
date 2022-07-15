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

fn imp_session_handle_ref() -> Tokens {
    let imp = rust::import("horizon_ipc::cmif", "SessionHandleRef");

    quote!($imp)
}

fn imp_session_handle() -> Tokens {
    let imp = rust::import("horizon_ipc::cmif", "SessionHandle");

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

fn imp_ipc_buffer_repr() -> Tokens {
    let imp = rust::import("horizon_ipc::buffer", "IpcBufferRepr");

    quote!($imp)
}

fn imp_get_ipc_buffer_for() -> Tokens {
    let imp = rust::import("horizon_ipc::buffer", "get_ipc_buffer_for");

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
    RawValue,
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
    pub fn has_special_header(&self) -> bool {
        self.should_pass_pid || !self.handles_in.is_empty() || self.handles_out.is_empty()
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

    pub fn in_raw_data_struct(&self) -> Struct {
        raw_data_struct(
            self.raw_data_in
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
                        $(imp_raw_handle())
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
                        $(imp_raw_handle())
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
    let s = raw_data_struct(data.iter().map(|d| (d.name.clone(), d.ty.clone())));

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
        if w_info.has_special_header() {
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
    let pre_cmif_padding = (16 - (cmif_header_offset * 4) % 16) % 16;

    let raw_data_size = w_info.in_raw_data_struct().layout(ctx).size();

    let raw_data_word_padding = (4 - (raw_data_size % 4)) % 4;

    let r: Tokens = quote! {
        #[repr(packed)]
        struct Request {
            hipc: $(imp_hipc_header()),
            $(if w_info.has_special_header() {
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

        // SAFETY: we checked the size before, so it should fit
        unsafe impl $(imp_ipc_buffer_repr()) for Request {}
    };

    r
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
                $(CommandType::Request as u32),
                $(in_pointer_buffers.len()),
                $(in_map_aliases.len()),
                $(out_map_aliases.len()),
                0, // num_inout_map_aliases
                $(sizes.data_size / 4), // num_data_words
                $(out_pointer_mode),
                0, // recv_list_offset
                $(if w_info.has_special_header() {
                    true
                } else {
                    false
                }),
            ),
            $(if w_info.has_special_header() {
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
            $(for (i, _) in in_pointer_buffers.iter().enumerate() {
                $(format!("in_pointer_desc_{}", i)): todo!(),
            })
            $(for (i, _) in in_map_aliases.iter().enumerate() {
                $(format!("in_map_alias_desc_{}", i)): todo!(),
            })
            $(for (i, _) in out_map_aliases.iter().enumerate() {
                $(format!("out_map_alias_desc_{}", i)): todo!(),
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

            $(for (i, _) in out_pointer_buffers.iter().enumerate() {
                $(format!("out_pointer_desc_{}", i)): todo!(),
            })
        }
    };

    r
}

pub enum CommandType {
    Invalid = 0,
    LegacyRequest = 1,
    Close = 2,
    LegacyControl = 3,
    Request = 4,
    Control = 5,
    RequestWithContext = 6,
    ControlWithContext = 7,
}

fn make_command_body(
    namespace: &Namespace,
    ctx: &CodegenContext,
    i_info: &CommandInterfaceInfo,
    w_info: &CommandWireFormatInfo,
) -> Tokens {
    let CommandInterfaceInfo { uninit_vars, .. } = i_info;
    let CommandWireFormatInfo {
        is_domain: _,
        command_id: _,
        buffers: _,
        raw_data_in,
        raw_data_out,
        handles_in: _,
        handles_out: _,
        should_pass_pid: _,
    } = w_info;

    let r: Tokens = quote! {
        // defines a data_in variable
        $(make_raw_data_in(namespace, ctx, &raw_data_in))

        $(make_request_struct(namespace, ctx, w_info))

        $(for (name, ty) in uninit_vars {
            let $(name.as_str()) = $(imp_maybe_uninit())::<$ty>::uninit();
        })

        // SAFETY: The pointer should be valid and has
        unsafe {
            ::core::ptr::write(
                $(imp_get_ipc_buffer_for())(),
                $(make_request(ctx, w_info))
            )
        };

        // TODO: process output
        $(if raw_data_out.is_empty() {
        } else {
            // $(make_raw_data_struct(
            //     namespace,
            //     ctx,
            //     Direction::Out,
            //     raw_data_out
            //         .iter()
            //         .map(|d| (d.name.clone(), d.ty.clone()))
            // ))
        })

        horizon_svc::send_sync_request(self.handle.0)?;

        todo!("Command codegen")
    };

    r
}

fn gen_command_in(
    namespace: &Namespace,
    tok: &mut Tokens,
    ctx: &CodegenContext,
    c: &Command,
    is_domain: bool,
) {
    let (i_info, w_info) = collect_command_info(namespace, ctx, is_domain, c);

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
    let name = c.name.to_case(Case::Snake);
    quote_in! { *tok =>
        pub fn $name(
            &self,
            $(for (name, ty) in &i_info.args join (,) => $(name.as_str()): $ty)
        ) -> $(imp_result())<$return_type> {
            $(make_command_body(namespace, ctx, &i_info, &w_info))
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
                pub(crate) handle: $(imp_session_handle()),
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
