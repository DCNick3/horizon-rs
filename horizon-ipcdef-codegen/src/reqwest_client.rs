use app_dirs2::AppDataType;
use once_cell::sync::Lazy;
use reqwest::{Client, IntoUrl};
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_middleware_cache::managers::CACacheManager;
use reqwest_middleware_cache::{Cache, CacheMode};

static REQWEST_CLIENT: Lazy<ClientWithMiddleware> = Lazy::new(|| {
    ClientBuilder::new(Client::new())
        .with(Cache {
            mode: CacheMode::Default,
            cache_manager: CACacheManager {
                path: app_dirs2::app_dir(
                    AppDataType::UserCache,
                    &crate::APP_INFO,
                    "reqwest-cacache",
                )
                .unwrap()
                .to_str()
                .unwrap()
                .to_string(),
            },
        })
        .build()
});

pub fn get<U: IntoUrl>(url: U) -> String {
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
