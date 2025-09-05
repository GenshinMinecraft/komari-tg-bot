use crate::connection::create_reqwest_client;
use crate::db::{query_monitor_by_telegram_id, DB_POOL};
use crate::ErrorString;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct ApiNodes {
    pub status: String,
    pub data: Vec<ApiNodesData>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ApiNodesData {
    pub uuid: String,
    pub name: String,
    pub cpu_name: String,
    pub virtualization: String,
    pub arch: String,
    pub cpu_cores: i32,
    pub os: String,
    pub kernel_version: String,
    pub gpu_name: String,
    pub region: String,
    pub mem_total: u64,
    pub swap_total: u64,
    pub disk_total: u64,
    pub price: Option<f64>,
    pub expired_at: Option<String>,
    pub group: Option<String>,
    pub tags: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

pub async fn get_api_nodes(telegram_id: i64) -> Result<ApiNodes, ErrorString> {
    let client = create_reqwest_client().await?;

    let monitor = query_monitor_by_telegram_id(
        DB_POOL
            .get()
            .unwrap_or_else(|| panic!("数据库连接池未初始化")),
        telegram_id,
    )
        .await?;

    let url = if let Some(monitor) = monitor {
        format!("{}/api/nodes", monitor.monitor_http_url)
    } else {
        return Err(ErrorString::from(
            "服务器未连接，请先使用 /connect [http url] 连接".to_string(),
        ));
    };

    let res = client
        .get(url)
        .send()
        .await
        .map_err(|e| ErrorString::from(e.to_string()))?;

    if res.status().as_u16() == 401 {
        return Err(ErrorString::from(String::from("主控开启了私有模式")));
    }

    if !res.status().is_success() {
        return Err(ErrorString::from(format!(
            "服务器返回错误：{}",
            res.status()
        )));
    }

    let text = res
        .text()
        .await
        .map_err(|e| ErrorString::from(format!("Text 解析错误: {e}")))?
        .trim()
        .to_string();

    let json = serde_json::from_str::<ApiNodes>(&text)
        .map_err(|e| ErrorString::from(format!("JSON 解析错误: {e}")))?;

    if json.status != "success" {
        return Err(ErrorString::from(format!(
            "服务器返回错误：{}",
            json.status
        )));
    }

    Ok(json)
}
