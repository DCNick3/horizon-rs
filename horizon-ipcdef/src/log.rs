use horizon_svc::RawHandle;
use std::fmt::{Display, Formatter};

struct HexDump<'a> {
    buffer: &'a [u8],
}

impl Display for HexDump<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        assert_eq!(self.buffer.len() % 4, 0);
        for w in self.buffer.chunks(4) {
            let w: [u8; 4] = w.try_into().unwrap();

            write!(f, "{:02x}{:02x}{:02x}{:02x} ", w[0], w[1], w[2], w[3])?;
        }
        Ok(())
    }
}

fn hex_dump(buffer: &[u8]) -> HexDump {
    HexDump { buffer }
}

pub fn pre_ipc_hook(name: &str, _handle: RawHandle) {
    let buffer = unsafe { horizon_ipc::buffer::get_ipc_buffer() };
    let name = format!("[{}]", name);
    eprintln!("{:50} IPC CALL   = {}", name, hex_dump(buffer));
}

pub fn post_ipc_hook(name: &str, _handle: RawHandle) {
    let buffer = unsafe { horizon_ipc::buffer::get_ipc_buffer() };
    let name = format!("[{}]", name);
    eprintln!("{:50} IPC RESULT = {}", name, hex_dump(buffer));
}
