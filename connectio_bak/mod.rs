pub mod api_nodes;
pub mod api_public;
pub mod api_version;
pub mod ws_get;

use crate::db::{delete_monitor, insert_monitor, query_monitor_by_telegram_id, Monitor, DB_POOL};
use crate::ErrorString;
use reqwest::Client;
use teloxide::types::Message;
use tokio::sync::OnceCell;

pub static REQWEST_CLIENT: OnceCell<reqwest::Client> = OnceCell::const_new();

pub async fn create_reqwest_client() -> Result<&'static Client, String> {
    REQWEST_CLIENT
        .get_or_try_init(|| async {
            let client_build = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .user_agent("komari-tgbot-rs");
            client_build.build()
        })
        .await
        .map_err(|e| ErrorString::from(e.to_string()))
}

pub async fn first_init_read(msg: Message) -> Result<String, ErrorString> {
    let db_pool = DB_POOL
        .get()
        .unwrap_or_else(|| panic!("数据库连接池未初始化"));

    let telegram_id = if let Some(user) = msg.clone().from {
        user.id.0 as i64
    } else {
        return Err(String::from("无法获取用户ID"));
    };

    let (public, nodes, version) = tokio::try_join!(
        api_public::get_api_public(telegram_id),
        api_nodes::get_api_nodes(telegram_id),
        api_version::get_api_version(telegram_id)
    )?;

    let site_name = public.data.sitename;
    let site_description = public.data.description;
    let version = format!("{}-{}", version.data.version, version.data.hash);
    let nodes_count = nodes.data.len();
    let cores_count = nodes.data.iter().map(|node| node.cpu_cores).sum::<i32>();
    let memory_total = nodes
        .data
        .iter()
        .map(|node| node.mem_total as f64 / 1024.0 / 1024.0 / 1024.0)
        .sum::<f64>();
    let swap_total = nodes
        .data
        .iter()
        .map(|node| node.swap_total as f64 / 1024.0 / 1024.0 / 1024.0)
        .sum::<f64>();
    let disk_total = nodes
        .data
        .iter()
        .map(|node| node.disk_total as f64 / 1024.0 / 1024.0 / 1024.0)
        .sum::<f64>();

    let Some(monitor_bak) = query_monitor_by_telegram_id(db_pool, telegram_id).await? else {
        return Ok("未找到该用户".to_string());
    };

    delete_monitor(db_pool, msg.clone()).await?;

    if let Err(e) = insert_monitor(
        db_pool,
        Monitor {
            telegram_id: monitor_bak.telegram_id,
            monitor_http_url: monitor_bak.monitor_http_url,
            monitor_ws_url: monitor_bak.monitor_ws_url,
            total_server_count: nodes_count as u32,
            site_name: site_name.clone(),
            site_description: site_description.clone(),
            komari_version: version.clone(),
            notification_token: None,
        },
    )
        .await
    {
        return Err(format!("无法更新数据库: {e}"));
    }

    Ok(format!(
        "成功读取 Komari 服务信息！\n\
         站点名称：`{site_name}`\n\
         站点详情：`{site_description}`\n\
         Komari 版本：`{version}`\n\
         节点数量：`{nodes_count}`\n\
         CPU 核心总数：`{cores_count}`\n\
         内存总量：`{memory_total:.2} GiB`\n\
         交换分区总量：`{swap_total:.2} GiB`\n\
         硬盘总量：`{disk_total:.2} GiB`"
    ))
}

pub fn msg_fixer(msg: String) -> String {
    msg.replace('.', r"\.")
        .replace('-', r"\-")
        .replace('|', r"\|")
        .replace('(', r"\(")
        .replace(')', r"\)")
        .replace('#', r"\#")
        .replace('+', r"\+")
        .replace('=', r"\=")
        .replace('{', r"\{")
        .replace('}', r"\}")
        .replace('[', r"\[")
        .replace(']', r"\]")
        .replace('_', r"\_")
        .replace('>', r"\>")
        .replace('<', r"\<")
        .replace('&', r"\&")
        .replace('!', r"\!")
}
