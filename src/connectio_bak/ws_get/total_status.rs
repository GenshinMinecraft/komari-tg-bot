use crate::connection::msg_fixer;
use crate::connection::ws_get::{get_ws, ApiWs};
use crate::db::{query_monitor_by_telegram_id, DB_POOL};
use crate::{connection, ErrorString};
use tokio::task::JoinHandle;

pub async fn parse_ws_total_status(telegram_id: i64) -> Result<String, ErrorString> {
    let ws_handle = tokio::spawn(async move {
        let ws_data = match get_ws(telegram_id).await {
            Ok(ws_data) => ws_data,
            Err(e) => {
                return Err(format!("无法连接到 Komari Websocket 服务器: {e}"));
            }
        };
        Ok(ws_data)
    });

    let http_handle: JoinHandle<Result<connection::api_nodes::ApiNodes, ErrorString>> =
        tokio::spawn(async move {
            let nodes = connection::api_nodes::get_api_nodes(telegram_id).await?;
            Ok(nodes)
        });

    let (ws_data, nodes): (
        Result<ApiWs, ErrorString>,
        Result<connection::api_nodes::ApiNodes, ErrorString>,
    ) = tokio::try_join!(ws_handle, http_handle)
        .map_err(|e| format!("无法运行 Tokio 线程: {e}"))?;

    let ws_data = ws_data?;
    let nodes = nodes?;

    let monitor = query_monitor_by_telegram_id(
        DB_POOL
            .get()
            .unwrap_or_else(|| panic!("数据库连接池未初始化")),
        telegram_id,
    )
        .await?
        .ok_or(String::from(
            "服务器未连接，请先使用 /connect [http url] 连接",
        ))?;

    let online_nodes_count = ws_data.data.online.len();
    let total_nodes_count = monitor.total_server_count;

    let cores_count = nodes.data.iter().map(|node| node.cpu_cores).sum::<i32>();

    let percent_online = online_nodes_count as f64 / f64::from(total_nodes_count) * 100.0;

    let avg_cpu_usage = ws_data
        .data
        .data
        .values()
        .map(|node| node.cpu.usage)
        .sum::<f64>()
        / ws_data.data.data.len() as f64;

    let total_total_ram = ws_data
        .data
        .data
        .values()
        .map(|node| node.ram.total)
        .sum::<u64>() as f64
        / 1024.0
        / 1024.0
        / 1024.0;
    let total_used_ram = ws_data
        .data
        .data
        .values()
        .map(|node| node.ram.used)
        .sum::<u64>() as f64
        / 1024.0
        / 1024.0
        / 1024.0;
    let avg_ram_usage = total_used_ram / total_total_ram * 100.0;

    let total_total_swap = ws_data
        .data
        .data
        .values()
        .map(|node| node.swap.total)
        .sum::<u64>() as f64
        / 1024.0
        / 1024.0
        / 1024.0;
    let total_used_swap = ws_data
        .data
        .data
        .values()
        .map(|node| node.swap.used)
        .sum::<u64>() as f64
        / 1024.0
        / 1024.0
        / 1024.0;
    let avg_swap_usage = total_used_swap / total_total_swap * 100.0;

    let total_total_disk = ws_data
        .data
        .data
        .values()
        .map(|node| node.disk.total)
        .sum::<u64>() as f64
        / 1024.0
        / 1024.0
        / 1024.0;
    let total_used_disk = ws_data
        .data
        .data
        .values()
        .map(|node| node.disk.used)
        .sum::<u64>() as f64
        / 1024.0
        / 1024.0
        / 1024.0;
    let avg_disk_usage = total_used_disk / total_total_disk * 100.0;

    let avg_load1 = ws_data
        .data
        .data
        .values()
        .map(|node| node.load.load1)
        .sum::<f64>()
        / ws_data.data.data.len() as f64;
    let avg_load5 = ws_data
        .data
        .data
        .values()
        .map(|node| node.load.load5)
        .sum::<f64>()
        / ws_data.data.data.len() as f64;
    let avg_load15 = ws_data
        .data
        .data
        .values()
        .map(|node| node.load.load15)
        .sum::<f64>()
        / ws_data.data.data.len() as f64;

    let total_total_net_down = ws_data
        .data
        .data
        .values()
        .map(|node| node.network.total_down)
        .sum::<u64>() as f64
        / 1024.0
        / 1024.0
        / 1024.0;
    let total_total_net_up = ws_data
        .data
        .data
        .values()
        .map(|node| node.network.total_up)
        .sum::<u64>() as f64
        / 1024.0
        / 1024.0
        / 1024.0;
    let total_net_down = ws_data
        .data
        .data
        .values()
        .map(|node| node.network.down)
        .sum::<u64>() as f64
        / 125000.0;
    let total_net_up = ws_data
        .data
        .data
        .values()
        .map(|node| node.network.up)
        .sum::<u64>() as f64
        / 125000.0;

    let total_tcp_connections = ws_data
        .data
        .data
        .values()
        .map(|node| node.connections.tcp)
        .sum::<u32>();
    let total_udp_connections = ws_data
        .data
        .data
        .values()
        .map(|node| node.connections.udp)
        .sum::<u32>();

    let title = monitor.site_name;

    Ok(msg_fixer(format!(
        r"{title} 总览

ONLINE: `{online_nodes_count}` / `{total_nodes_count}` `{percent_online:.2}%`
CPU CORES: `{cores_count}`
AVG CPU: `{avg_cpu_usage:.2}%`
AVG LOAD: `{avg_load1:.2}` / `{avg_load5:.2}` / `{avg_load15:.2}`

MEM: `{total_used_ram:.2} GB` / `{total_total_ram:.2} GB` `{avg_ram_usage:.2}%`
SWAP: `{total_used_swap:.2} GB` / `{total_total_swap:.2} GB` `{avg_swap_usage:.2}%`
DISK: `{total_used_disk:.2} GB` / `{total_total_disk:.2} GB` `{avg_disk_usage:.2}%`

DOWN: `{total_total_net_down:.2} GB`
UP: `{total_total_net_up:.2} GB`
DOWN SPEED: `{total_net_down:.2} Mbps`
UP SPEED: `{total_net_up:.2} Mbps`
CONN: `{total_tcp_connections} TCP` / `{total_udp_connections} UDP`"
    )))
}
