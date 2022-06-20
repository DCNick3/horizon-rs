
typedef unsigned u32;

typedef struct HipcHeader {
    u32 type               : 16;
    u32 num_send_statics   : 4;
    u32 num_send_buffers   : 4;
    u32 num_recv_buffers   : 4;
    u32 num_exch_buffers   : 4;
    u32 num_data_words     : 10;
    u32 recv_static_mode   : 4;
    u32 padding            : 6;
    u32 recv_list_offset   : 11; // Unused.
    u32 has_special_header : 1;
} HipcHeader;

typedef struct HipcSpecialHeader {
    u32 send_pid         : 1;
    u32 num_copy_handles : 4;
    u32 num_move_handles : 4;
    u32 padding          : 23;
} HipcSpecialHeader;

typedef struct HipcStaticDescriptor {
    u32 index        : 6;
    u32 address_high : 6;
    u32 address_mid  : 4;
    u32 size         : 16;
    u32 address_low;
} HipcStaticDescriptor;

typedef struct HipcBufferDescriptor {
    u32 size_low;
    u32 address_low;
    u32 mode         : 2;
    u32 address_high : 22;
    u32 size_high    : 4;
    u32 address_mid  : 4;
} HipcBufferDescriptor;

typedef struct HipcRecvListEntry {
    u32 address_low;
    u32 address_high : 16;
    u32 size         : 16;
} HipcRecvListEntry;