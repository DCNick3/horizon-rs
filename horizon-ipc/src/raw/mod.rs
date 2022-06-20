//! Contains raw types for Horizon IPC
//!
//! HIPC types for bitfields generated via bindgen:
//!  bindgen --use-core --ctypes-prefix super::c_types --no-layout-tests hipc.h -o hipc.rs

mod c_types;

#[allow(dead_code)]
pub mod cmif;
#[allow(non_camel_case_types, dead_code)]
pub mod hipc;
