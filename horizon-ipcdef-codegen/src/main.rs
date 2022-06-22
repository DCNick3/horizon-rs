mod ipc_parse;

use crate::ipc_parse::IpcFile;
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

const NINUPDATES_BASE_URL: &str = "https://yls8.mtheall.com/ninupdates";

const NINUPDATES_DATE: &str = "2022-03-22_00-05-06";

static FILE_NAME_FORMAT: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"sysupdatedl/autodl_sysupdates/(?P<date>[^/]*)/(?P<title_id>[^/]*)/(?P<region>[^/]*)/v(?P<version>[^/]*)/(?P<file>[^/]*)",
    )
    .unwrap()
});

static REQWEST_CLIENT: Lazy<ClientWithMiddleware> = Lazy::new(|| {
    ClientBuilder::new(Client::new())
        .with(Cache {
            mode: CacheMode::Default,
            cache_manager: CACacheManager::default(),
        })
        .build()
});

fn get<U: IntoUrl>(url: U) -> String {
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        REQWEST_CLIENT
            .get(url)
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap()
    })
}

#[derive(Eq, PartialEq)]
enum Region {
    Global,
    China,
}

impl Region {
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "CHN" => Region::China,
            "ALL" => Region::Global,
            _ => return None,
        })
    }
}

impl Display for Region {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Region::Global => "ALL",
            Region::China => "CHN",
        };
        write!(f, "{}", s)
    }
}

struct FileId {
    pub date: String,
    pub title_id: u64,
    pub region: Region,
    pub version: u32,
    pub filename: String,
}

impl FileId {
    pub fn parse(s: &str) -> Option<Self> {
        let c = FILE_NAME_FORMAT.captures(s)?;

        Some(Self {
            date: c.name("date").unwrap().as_str().to_string(),
            title_id: u64::from_str_radix(c.name("title_id").unwrap().as_str(), 16).unwrap(),
            region: Region::parse(c.name("region").unwrap().as_str())?,
            version: u32::from_str(&c.name("version").unwrap().as_str()).unwrap(),
            filename: c.name("file").unwrap().as_str().to_string(),
        })
    }

    pub fn get(&self) -> String {
        let url = format!("{}/{}", NINUPDATES_BASE_URL, self);

        get(url)
    }
}

impl Debug for FileId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "sysupdatedl/autodl_sysupdates/{}/{:016X}/{}/v{}/{}",
            self.date, self.title_id, self.region, self.version, self.filename
        )
    }
}

impl Display for FileId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

fn get_file_list() -> Vec<FileId> {
    let url = format!(
        "{}/titlelist.php?date={}&sys=hac&reg=G",
        NINUPDATES_BASE_URL, NINUPDATES_DATE
    );

    let list = get(&url);

    let html = Html::parse_document(&list);

    let mut res = Vec::new();

    for link in html.select(&Selector::parse("a").unwrap()) {
        if let Some(href) = link.value().attr("href") {
            // TODO: __maybe__ we will need info that is not title-specific (usually has "alltitles" in the file name)
            // ignore for now
            if let Some(file_id) = FileId::parse(href) {
                res.push(file_id);
            } else {
                if href.starts_with("sysupdatedl") {
                    eprintln!(
                        "Skipping potential file due to unsupported path format: {}",
                        href
                    )
                }
            }
        }
    }

    res
}

fn main() {
    let files = get_file_list();

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

        let r = IpcFile::parse(&contents);

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
