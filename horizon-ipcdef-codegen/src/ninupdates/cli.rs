use crate::{ninupdates, Region};
use std::collections::HashSet;

/// Fetch some data from nintupdates, IDK
/// Does not actually does anything useful yet, more like a test
#[derive(clap::Args, Debug)]
pub struct Args {}

pub fn run(_args: Args) -> anyhow::Result<()> {
    let files = ninupdates::get_file_list();

    let files = files
        .into_iter()
        .filter(|f| {
            f.region == Region::Global && f.filename.ends_with("swipcgen_server_modern.info")
        })
        .collect::<Vec<_>>();

    let mut buffer_types = HashSet::new();

    for file in files {
        // println!("=======");
        println!("{}", file);
        let contents = file.get();

        let r = ninupdates::ipc_parse::IpcFile::parse(&contents);

        if let Ok(ipc) = r {
            for iface in ipc.interfaces {
                for (_, method) in iface.methods {
                    buffer_types.extend(method.buffers.into_iter())
                }
            }
        }

        // println!("{:#?}", r);
        // println!();
        // println!();
    }

    println!("buffer_types = {:#?}", buffer_types);

    Ok(())
}
