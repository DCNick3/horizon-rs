mod ninupdates;
pub mod reqwest_client;

use crate::ninupdates::Region;
use app_dirs2::{AppDataType, AppInfo};
use once_cell::sync::Lazy;
use regex::Regex;
use reqwest::{Client, IntoUrl};
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_middleware_cache::managers::CACacheManager;
use reqwest_middleware_cache::{Cache, CacheMode};
use scraper::{Html, Selector};
use std::collections::HashSet;
use std::fmt::{Debug, Display, Formatter};
use std::str::FromStr;

const APP_INFO: AppInfo = AppInfo {
    name: "horizon-ipcdef-codegen",
    author: "DCNick3",
};

fn main() {
    let files = crate::ninupdates::get_file_list();

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

    println!("buffer_types = {:#?}", buffer_types)
}
