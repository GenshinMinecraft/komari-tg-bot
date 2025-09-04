use crate::ErrorString;
use crate::connection::api_nodes::{ApiNodes, get_api_nodes};
use crate::connection::msg_fixer;
use crate::connection::ws_get::{ApiWs, ApiWsDataHashMapValue, get_ws};
use crate::db::{DB_POOL, query_monitor_by_telegram_id};
use reqwest::Url;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};
use tokio::task::JoinHandle;

pub async fn ws_single_server_by_index(
    telegram_id: i64,
    index: i32,
) -> Result<Option<(String, ApiWsDataHashMapValue)>, ErrorString> {
    let ws_data = match get_ws(telegram_id).await {
        Ok(ws_data) => ws_data,
        Err(e) => {
            return Err(format!("无法连接到 Komari Websocket 服务器: {e}"));
        }
    };

    let sorted_data = sort_ws_data(ws_data);

    if index > sorted_data.len() as i32 {
        return Ok(None);
    }

    let vec_index: usize = match index {
        0 | 1 => 0,
        _ => (index - 1) as usize,
    };

    Ok(sorted_data.get(vec_index).cloned())
}

pub fn sort_ws_data(ws_data: ApiWs) -> Vec<(String, ApiWsDataHashMapValue)> {
    let mut sorted_data: Vec<(String, ApiWsDataHashMapValue)> =
        ws_data.data.data.into_iter().collect();
    sorted_data.sort_by(|a, b| a.0.cmp(&b.0));
    sorted_data
}

pub async fn parse_ws_single_server_by_index(
    telegram_id: i64,
    index: i32,
) -> Result<String, ErrorString> {
    let ws_handle = tokio::spawn(async move {
        let Some(node) = ws_single_server_by_index(telegram_id, index).await? else {
            return Err(String::from("找不到该序号的服务器"));
        };
        Ok(node)
    });
    let http_handle: JoinHandle<Result<ApiNodes, ErrorString>> = tokio::spawn(async move {
        let nodes = get_api_nodes(telegram_id).await?;
        Ok(nodes)
    });
    let monitor = query_monitor_by_telegram_id(
        DB_POOL
            .get()
            .unwrap_or_else(|| panic!("数据库连接池未初始化")),
        telegram_id,
    )
    .await?
    .ok_or(String::from(
        "服务器未连接，请先使用 /connect [http url] [ws url] 连接",
    ))?;

    let (ws_data, nodes) = tokio::try_join!(ws_handle, http_handle)
        .map_err(|e| format!("无法运行 Tokio 线程: {e}"))?;
    let nodes = nodes?;

    let (uuid, ws_data) = ws_data?;

    let node = nodes
        .data
        .iter()
        .find(|node| node.uuid == uuid)
        .ok_or("找不到该序号的服务器")?;

    let title = monitor.site_name;
    let region = node.region.clone();
    let name = node.name.clone();
    let updated_at = node.updated_at.clone();
    let processes = ws_data.process;

    let cpu_name = node.cpu_name.clone();
    let cpu_cores = node.cpu_cores;
    let os = node.os.clone();
    let kernel_version = node.kernel_version.clone();
    let virtualization = node.virtualization.clone();
    let arch = node.arch.clone();
    let cpu_usage = ws_data.cpu.usage;

    let gpu_name = node.gpu_name.clone();

    let ram_total = ws_data.ram.total as f64 / 1024.0 / 1024.0;
    let ram_used = ws_data.ram.used as f64 / 1024.0 / 1024.0;
    let ram_usage = ram_used / ram_total * 100.0;

    let swap_total = ws_data.swap.total as f64 / 1024.0 / 1024.0;
    let swap_used = ws_data.swap.used as f64 / 1024.0 / 1024.0;
    let swap_usage = swap_used / swap_total * 100.0;

    let disk_total = ws_data.disk.total as f64 / 1024.0 / 1024.0 / 1024.0;
    let disk_used = ws_data.disk.used as f64 / 1024.0 / 1024.0 / 1024.0;
    let disk_usage = disk_used / disk_total * 100.0;

    let uptime = format_duration(ws_data.uptime);
    let load1 = ws_data.load.load1;
    let load5 = ws_data.load.load5;
    let load15 = ws_data.load.load15;

    let total_net_down = ws_data.network.total_down as f64 / 1024.0 / 1024.0 / 1024.0;
    let total_net_up = ws_data.network.total_up as f64 / 1024.0 / 1024.0 / 1024.0;
    let net_down = ws_data.network.down as f64 / 125000.0;
    let net_up = ws_data.network.up as f64 / 125000.0;

    let total_tcp_connections = ws_data.connections.tcp;
    let total_udp_connections = ws_data.connections.udp;

    let source_str = format!(
        r"{title} | {region} | {name}

CPU: `{cpu_name}` @ `{cpu_cores} Cores`{}
ARCH: `{arch}`
VIRT: `{virtualization}`
OS: `{os}`
KERN: `{kernel_version}`
UPTIME: `{uptime}`

CPU: `{cpu_usage:.2}%`
RAM: `{ram_used:.2}` / `{ram_total:.2} MB` `{ram_usage:.2}%`
SWAP: `{swap_used:.2}` / `{swap_total:.2} MB` `{swap_usage:.2}%`
DISK: `{disk_used:.2}` / `{disk_total:.2} GB` `{disk_usage:.2}%`

LOAD: `{load1:.2}` / `{load5:.2}` / `{load15:.2}`
PROC: `{processes}`

NET: `{total_net_down:.2} GB` / `{total_net_up:.2} GB`
UP: `{net_down:.2} Mbps`
DOWN: `{net_up:.2} Mbps`
CONN: `{total_tcp_connections} TCP` / `{total_udp_connections} UDP`{}",
        {
            if gpu_name.is_empty() {
                String::new()
            } else {
                format!(
                    "
GPU: `{gpu_name}`"
                )
            }
        },
        {
            match updated_at {
                Some(updated_at) => {
                    format!(
                        "

UPDATE AT: `{updated_at}`"
                    )
                }
                None => String::new(),
            }
        }
    );

    Ok(msg_fixer(source_str))
}

pub fn format_duration(mut seconds: u64) -> String {
    if seconds == 0 {
        return "0 秒".to_string();
    }

    let mut result = Vec::new();

    let time_units = [
        (31536000, "年"), // 365 * 24 * 60 * 60
        (2592000, "月"),  // 30 * 24 * 60 * 60
        (86400, "天"),    // 24 * 60 * 60
        (3600, "时"),     // 60 * 60
        (60, "分"),       // 60
        (1, "秒"),        // 1
    ];

    for (unit_seconds, unit_name) in &time_units {
        let value = seconds / unit_seconds;
        if value > 0 {
            result.push(format!("{value} {unit_name}"));
        }
        seconds %= unit_seconds;
    }

    result.join(" ")
}

pub async fn make_keyboard_for_single(
    now_id: i32,
    telegram_id: i64,
) -> Result<InlineKeyboardMarkup, ErrorString> {
    let db_pool = DB_POOL
        .get()
        .unwrap_or_else(|| panic!("数据库连接池未初始化"));
    let monitor = query_monitor_by_telegram_id(db_pool, telegram_id)
        .await?
        .ok_or(String::from(
            "服务器未连接，请先使用 /connect [http url] [ws url] 添加服务器",
        ))?;
    let max_server = monitor.total_server_count;

    let mut keyboard: Vec<Vec<InlineKeyboardButton>> = vec![];
    let mut first_row = vec![];

    let send_id = match now_id {
        0 | 1 => (0, 2),
        _ => (now_id - 1, now_id + 1),
    };

    if send_id.0 > 0 {
        first_row.push(InlineKeyboardButton::callback(
            "<-",
            format!("{}-{}", telegram_id, send_id.0),
        ));
    }

    first_row.push(InlineKeyboardButton::url(
        format!("{now_id} / {max_server}"),
        Url::parse("https://t.me/komaritgbot").unwrap(),
    ));

    if send_id.1 <= max_server as i32 {
        first_row.push(InlineKeyboardButton::callback(
            "->",
            format!("{}-{}", telegram_id, send_id.1),
        ));
    }

    keyboard.push(first_row);
    keyboard.push(vec![InlineKeyboardButton::callback(
        "Refresh",
        format!("{}-{}", telegram_id, now_id),
    )]);

    Ok(InlineKeyboardMarkup::new(keyboard))
}
