//! Contains raw types for Horizon IPC
//!
//! HIPC types for bitfields generated via bindgen:
//!  `bindgen --use-core --ctypes-prefix super::c_types --no-layout-tests hipc.h -o hipc.rs`
//!  `sed -i '/\#\[repr(align(4))\]/d' hipc.rs`

mod c_types;

#[allow(dead_code)]
pub mod cmif;
#[allow(non_camel_case_types, dead_code, clippy::too_many_arguments)]
pub mod hipc;
mod hipc_conv;
