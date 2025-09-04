pub mod get_node_id;
pub mod status;
pub mod total_status;

use crate::db::{query_monitor_by_telegram_id, DB_POOL};
use crate::ErrorString;
use futures::{SinkExt, StreamExt};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::handshake::client::{generate_key, Request};
use tokio_tungstenite::tungstenite::{Message, Utf8Bytes};
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

pub async fn connect_ws(
    http_url: &str,
    ws_url: &str,
) -> Result<WebSocketStream<MaybeTlsStream<TcpStream>>, ErrorString> {
    let host = Url::parse(ws_url).map_err(|_| "无法解析 URL".to_string())?;
    let host = host.host_str().ok_or("无法获取主机名".to_string())?;

    let request = Request::builder()
        .method("GET")
        .uri(format!("{ws_url}/api/clients"))
        .header("origin", http_url)
        .header("sec-websocket-key", generate_key())
        .header("host", host)
        .header("sec-websocket-version", "13")
        .header(
            "sec-websocket-extensions",
            "permessage-deflate; client_max_window_bits",
        )
        .header("upgrade", "websocket")
        .header("connection", "Upgrade")
        .body(())
        .map_err(|_| "无法创建 WebSocket 请求".to_string())?;

    match connect_async(request).await {
        Ok((ws_stream, _)) => Ok(ws_stream),
        Err(e) => Err(format!("无法连接到 Komari Websocket 服务器: {e}")),
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ApiWs {
    pub status: String,
    pub data: ApiWsData,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ApiWsData {
    pub online: Vec<String>,
    pub data: HashMap<String, ApiWsDataHashMapValue>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ApiWsDataHashMapValue {
    pub cpu: Cpu,
    pub ram: Ram,
    pub swap: Swap,
    pub load: Load,
    pub disk: Disk,
    pub network: Network,
    pub connections: Connections,
    pub uptime: u64,
    pub process: u32,
    pub message: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Cpu {
    pub usage: f64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Ram {
    pub total: u64,
    pub used: u64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Swap {
    pub total: u64,
    pub used: u64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Load {
    pub load1: f64,
    pub load5: f64,
    pub load15: f64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Disk {
    pub total: u64,
    pub used: u64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Network {
    pub up: u64,
    pub down: u64,
    #[serde(rename = "totalUp")]
    pub total_up: u64,
    #[serde(rename = "totalDown")]
    pub total_down: u64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Connections {
    pub tcp: u32,
    pub udp: u32,
}

pub async fn get_ws(telegram_id: i64) -> Result<ApiWs, ErrorString> {
    let monitor = query_monitor_by_telegram_id(
        DB_POOL
            .get()
            .unwrap_or_else(|| panic!("数据库连接池未初始化")),
        telegram_id,
    )
        .await?;

    let (http_url, ws_url) = if let Some(monitor) = monitor {
        (monitor.monitor_http_url, monitor.monitor_ws_url)
    } else {
        return Err(ErrorString::from(
            "服务器未连接，请先使用 /connect [http url] 连接".to_string(),
        ));
    };

    let ws_connection = connect_ws(&http_url, &ws_url).await?;

    let (mut write, mut read) = ws_connection.split();

    write
        .send(Message::Text(Utf8Bytes::from("get")))
        .await
        .map_err(|_| String::from("无法发送数据"))?;

    let Some(Ok(msg)) = read.next().await else {
        return Err(String::from("数据接收出现错误"));
    };

    let data_str = msg
        .to_text()
        .map_err(|_| String::from("无法将 Websocket 返回内容转化为文本"))?;

    let data: ApiWs = serde_json::from_str(data_str)
        .map_err(|e| format!("无法将 Websocket 响应内容转化为 JSON: {e}"))?;

    Ok(data)
}
