
typedef unsigned u32;
typedef unsigned short u16;
typedef unsigned char u8;

typedef struct HipcHeader {
    u16 type                  : 16;
    u32 num_in_pointers       : 4;
    u32 num_in_map_aliases    : 4;
    u32 num_out_map_aliases   : 4;
    u32 num_inout_map_aliases : 4;
    u32 num_data_words        : 10;
    u32 out_pointer_mode      : 4;
    u32 padding               : 6;
    u32 recv_list_offset      : 11; // Unused.
    u32 has_special_header    : 1;
} HipcHeader;

typedef struct HipcSpecialHeader {
    u32 send_pid         : 1;
    u32 num_copy_handles : 4;
    u32 num_move_handles : 4;
    u32 padding          : 23;
} HipcSpecialHeader;

typedef struct HipcInPointerBufferDescriptor {
    u32 index        : 6;
    u32 address_high : 6;
    u32 address_mid  : 4;
    u32 size         : 16;
    u32 address_low;
} HipcInPointerBufferDescriptor;

typedef struct HipcMapAliasBufferDescriptor {
    u32 size_low;
    u32 address_low;
    u32 mode         : 2;
    u32 address_high : 22;
    u32 size_high    : 4;
    u32 address_mid  : 4;
} HipcMapAliasBufferDescriptor;

typedef struct HipcOutPointerBufferDescriptor {
    u32 address_low;
    u32 address_high : 16;
    u32 size         : 16;
} HipcOutPointerBufferDescriptor;