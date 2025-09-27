pub mod all_komari_info;
pub mod connect;
pub mod get_node_id;
pub mod query;
pub mod status;
pub mod total_status;

use reqwest::Client;
use tokio::sync::OnceCell;
use crate::utils::ErrorType;

pub static REQWEST_CLIENT: OnceCell<reqwest::Client> = OnceCell::const_new();

pub async fn create_reqwest_client() -> Result<&'static Client, ErrorType> {
    REQWEST_CLIENT
        .get_or_try_init(|| async {
            let client_build = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .user_agent("komari-tgbot-rs");
            client_build.build()
        })
        .await
        .map_err(|e| ErrorType::UnableToCreateReqwestClient { error: e.to_string() })
}

pub fn bytes_to_pretty_string<T: Into<i64>>(bytes: T) -> String {
    let bytes = bytes.into() as f64;
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB", "PB", "EB", "ZB", "YB"];
    const DIVISOR: f64 = 1024.0;

    if bytes == 0.0 {
        return "0 B".to_string();
    }

    let mut size = bytes;
    let mut unit_index = 0;

    while size >= DIVISOR && unit_index < UNITS.len() - 1 {
        size /= DIVISOR;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", size as i64, UNITS[unit_index])
    } else {
        format!("{:.2} {}", size, UNITS[unit_index])
    }
}
