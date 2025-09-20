use crate::db::query_monitor_by_telegram_id;
use crate::json_rpc::create_reqwest_client;
use crate::{ErrorString, TelegramId, db};
use axum::{
    Router,
    extract::{Path, State},
};
use log::{error, info};
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;
use axum::routing::post;
use tokio::sync::Mutex;
use urlencoding::encode;

type CallbackFunc = fn(
    String,
    String,
    String,
    String,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>;

#[derive(Clone)]
struct AppState {
    callback: Arc<Mutex<CallbackFunc>>,
}

pub async fn http_callback(param1: String, param2: String, param3: String, body: String) {
    let Ok(telegram_id) = param1.parse::<i64>() else {
        info!("Webhook: 无法解析telegram_id: {param1}");
        return;
    };
    info!("Webhook: {telegram_id} {param1} {param2} {param3}");

    let db_pool = db::DB_POOL
        .get()
        .unwrap_or_else(|| panic!("数据库连接池未初始化"));

    let Ok(Some(monitor)) = query_monitor_by_telegram_id(db_pool, telegram_id).await else {
        error!("Webhook: 未找到telegram_id {telegram_id} 的监控信息");
        return;
    };

    if let Some(notification_token) = monitor.notification_token {
        if param2 != notification_token {
            error!("Webhook: 无效的token，期望: {notification_token}，实际: {param2}");
            return;
        }
    } else {
        error!("Webhook: telegram_id {telegram_id} 没有设置notification_token");
        return;
    }

    let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) else {
        error!("Webhook: 无法解析body为JSON: {body}");
        return;
    };

    let Ok(client) = create_reqwest_client().await else {
        error!("Webhook: 无法创建HTTP客户端");
        return;
    };

    let title = json
        .get("title")
        .and_then(|v| v.as_str())
        .map(std::string::ToString::to_string);
    let message = json
        .get("message")
        .and_then(|v| v.as_str())
        .map(std::string::ToString::to_string);

    let Some(title) = title else {
        error!("Webhook: 缺少title字段");
        return;
    };

    let Some(message) = message else {
        error!("Webhook: 缺少message字段");
        return;
    };

    let Ok(tg_token) = env::var("TG_TOKEN") else {
        error!("Webhook: 缺少TG_TOKEN环境变量");
        return;
    };

    let url = format!(
        "https://api.telegram.org/bot{tg_token}/sendMessage?chat_id={param3}&text={}",
        encode(format!("[{title}] {message}").as_str())
    );

    let Ok(resp) = client.get(url).send().await else {
        error!("Webhook: 发送Telegram消息失败");
        return;
    };

    match resp.text().await {
        Ok(text) => info!("Telegram: {text}"),
        Err(e) => error!("Webhook: 无法获取响应文本: {e}"),
    }
}

async fn telegram_handler(
    State(state): State<AppState>,
    Path((telegram_id, token, chat_id)): Path<(String, String, String)>,
    body: String,
) -> &'static str {
    let cb = state.callback.lock().await;

    (*cb)(telegram_id, token, chat_id, body).await;

    "OK"
}

pub async fn start_server(callback: CallbackFunc) {
    let shared_state = AppState {
        callback: Arc::new(Mutex::new(callback)),
    };
    let app = Router::new()
        .route(
            "/telegrambot/{telegram_id}/{token}/{chat_id}",
            post(telegram_handler),
        )
        .with_state(shared_state);

    let Ok(listen) = env::var("CALLBACK_HTTP_LISTEN") else {
        panic!("CALLBACK_HTTP_LISTEN not set");
    };

    let addr: SocketAddr = listen.parse().unwrap();
    info!("正在监听端口 http://{addr} ...");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

pub async fn generate_notification_token(telegram_id: TelegramId) -> Result<String, ErrorString> {
    let new_uuid = uuid::Uuid::new_v4().to_string();

    let db_pool = db::DB_POOL
        .get()
        .unwrap_or_else(|| panic!("数据库连接池未初始化"));

    db::update_notification_token(db_pool, telegram_id, new_uuid.clone())
        .await
        .map_err(|e| format!("无法更新数据库中的notification_token: {e}"))?;

    let Ok(callback_http_url) = env::var("CALLBACK_HTTP_URL") else {
        return Err("CALLBACK_HTTP_URL 未设置".to_string());
    };

    let body = r#"{"message":"{{message}}", "title":"{{title}}"}"#;
    Ok(format!(
        r"已生成新的 Uuid:
```
{new_uuid}
```
请使用以下链接作为 Callback URL:
```
{callback_http_url}/telegrambot/{telegram_id}/{new_uuid}/CHAT_ID
```
以下内容作为 Callback Body:
```
{body}
```

最后选择 Method 为 `Post` 并保存

请自行替换 CHAT\_ID，并确保该 Bot 可以访问到该聊天，CHAT\_ID 可从其他 Bot 获取"
    ))
}
