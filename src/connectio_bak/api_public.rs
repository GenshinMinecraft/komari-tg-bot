use crate::ErrorString;
use crate::connection::create_reqwest_client;
use crate::db::{DB_POOL, query_monitor_by_telegram_id};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct ApiPublic {
    pub status: String,
    pub data: ApiPublicData,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ApiPublicData {
    pub sitename: String,
    pub description: String,
}

pub async fn get_api_public(telegram_id: i64) -> Result<ApiPublic, ErrorString> {
    let client = create_reqwest_client().await?;

    let monitor = query_monitor_by_telegram_id(
        DB_POOL
            .get()
            .unwrap_or_else(|| panic!("数据库连接池未初始化")),
        telegram_id,
    )
    .await?;

    let url = if let Some(monitor) = monitor {
        format!("{}/api/public", monitor.monitor_http_url)
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

    let json = res
        .json::<ApiPublic>()
        .await
        .map_err(|e| ErrorString::from(format!("Json 解析错误: {e}")))?;

    if json.status != "success" {
        return Err(ErrorString::from(format!(
            "服务器返回错误：{}",
            json.status
        )));
    }

    Ok(json)
}
