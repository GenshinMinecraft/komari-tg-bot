use crate::db::{DB_POOL, query_monitor_by_telegram_id};
use crate::json_rpc::bytes_to_pretty_string;
use crate::json_rpc::query::AllInfo;
use crate::{ErrorString, MessageString, TelegramId};

pub async fn total_status(
    telegram_id: TelegramId,
) -> Result<(MessageString, AllInfo), ErrorString> {
    let db = DB_POOL.get().ok_or(String::from("无法获取数据库"))?;

    let Some(monitor) = query_monitor_by_telegram_id(db, telegram_id).await? else {
        return Err(ErrorString::from(
            "服务器未连接，请先使用 /connect [http url] 连接".to_string(),
        ));
    };

    let all_info = crate::json_rpc::query::get_all_info(&monitor.monitor_url).await?;

    let (online_nodes_count, total_nodes_count, percent_online) = {
        let online_nodes_count = all_info
            .common_nodes_latest_status
            .iter()
            .filter(|node| node.1.online)
            .count();

        let total_nodes_count = all_info.common_nodes_latest_status.len();

        let percent_online = if total_nodes_count > 0 {
            (online_nodes_count as f64 / total_nodes_count as f64) * 100.0
        } else {
            0.0
        };
        (online_nodes_count, total_nodes_count, percent_online)
    };

    let (avg_load1, avg_load5, avg_load15) = {
        let load1 = all_info
            .common_nodes_latest_status
            .values()
            .map(|node| node.load)
            .sum::<f64>()
            / online_nodes_count as f64;
        let load5 = all_info
            .common_nodes_latest_status
            .values()
            .map(|node| node.load5)
            .sum::<f64>()
            / online_nodes_count as f64;
        let load15 = all_info
            .common_nodes_latest_status
            .values()
            .map(|node| node.load15)
            .sum::<f64>()
            / online_nodes_count as f64;
        (load1, load5, load15)
    };

    let (total_used_ram, total_total_ram, avg_ram_usage) = {
        let total_used_ram = all_info
            .common_nodes_latest_status
            .values()
            .map(|node| node.ram)
            .sum::<i64>();
        let total_total_ram = all_info
            .common_nodes_latest_status
            .values()
            .map(|node| node.ram_total)
            .sum::<i64>();
        let avg_ram_usage = if total_total_ram > 0 {
            (total_used_ram as f64 / total_total_ram as f64) * 100.0
        } else {
            0.0
        };

        (
            bytes_to_pretty_string(total_used_ram),
            bytes_to_pretty_string(total_total_ram),
            avg_ram_usage,
        )
    };

    let (total_used_swap, total_total_swap, avg_swap_usage) = {
        let total_used_swap = all_info
            .common_nodes_latest_status
            .values()
            .map(|node| node.swap)
            .sum::<i64>();
        let total_total_swap = all_info
            .common_nodes_latest_status
            .values()
            .map(|node| node.swap_total)
            .sum::<i64>();
        let avg_swap_usage = if total_total_swap > 0 {
            (total_used_swap as f64 / total_total_swap as f64) * 100.0
        } else {
            0.0
        };

        (
            bytes_to_pretty_string(total_used_swap),
            bytes_to_pretty_string(total_total_swap),
            avg_swap_usage,
        )
    };

    let (total_used_disk, total_total_disk, avg_disk_usage) = {
        let total_used_disk = all_info
            .common_nodes_latest_status
            .values()
            .map(|node| node.disk)
            .sum::<i64>();
        let total_total_disk = all_info
            .common_nodes_latest_status
            .values()
            .map(|node| node.disk_total)
            .sum::<i64>();
        let avg_disk_usage = if total_total_disk > 0 {
            (total_used_disk as f64 / total_total_disk as f64) * 100.0
        } else {
            0.0
        };

        (
            bytes_to_pretty_string(total_used_disk),
            bytes_to_pretty_string(total_total_disk),
            avg_disk_usage,
        )
    };

    let (
        total_total_net_down,
        total_total_net_up,
        total_net_down,
        total_net_up,
        total_tcp_connections,
        total_udp_connections,
    ) = {
        let total_total_net_down = all_info
            .common_nodes_latest_status
            .values()
            .map(|node| node.net_total_down)
            .sum::<i64>();
        let total_total_net_up = all_info
            .common_nodes_latest_status
            .values()
            .map(|node| node.net_total_up)
            .sum::<i64>();
        let total_net_down = all_info
            .common_nodes_latest_status
            .values()
            .map(|node| node.net_in)
            .sum::<i64>();
        let total_net_up = all_info
            .common_nodes_latest_status
            .values()
            .map(|node| node.net_out)
            .sum::<i64>();
        let total_tcp_connections = all_info
            .common_nodes_latest_status
            .values()
            .map(|node| node.connections)
            .sum::<i64>();
        let total_udp_connections = all_info
            .common_nodes_latest_status
            .values()
            .map(|node| node.connections_udp)
            .sum::<i64>();

        (
            bytes_to_pretty_string(total_total_net_down),
            bytes_to_pretty_string(total_total_net_up),
            total_net_down as f64 / 125000.0,
            total_net_up as f64 / 125000.0,
            total_tcp_connections,
            total_udp_connections,
        )
    };

    let msg = format!(
        r"{title} 总览

ONLINE: `{online_nodes_count}` / `{total_nodes_count}` `{percent_online:.2}%`
CPU CORES: `{cores_count}`
AVG CPU: `{avg_cpu_usage:.2}%`
AVG LOAD: `{avg_load1:.2}` / `{avg_load5:.2}` / `{avg_load15:.2}`

MEM: `{total_used_ram}` / `{total_total_ram}` `{avg_ram_usage:.2}%`
SWAP: `{total_used_swap}` / `{total_total_swap}` `{avg_swap_usage:.2}%`
DISK: `{total_used_disk}` / `{total_total_disk}` `{avg_disk_usage:.2}%`

DOWN: `{total_total_net_down}`
UP: `{total_total_net_up}`
DOWN SPEED: `{total_net_down:.2} Mbps`
UP SPEED: `{total_net_up:.2} Mbps`
CONN: `{total_tcp_connections} TCP` / `{total_udp_connections} UDP`",
        title = all_info.common_public_info.sitename,
        cores_count = all_info
            .common_nodes
            .values()
            .map(|node| node.cpu_cores)
            .sum::<i64>(),
        avg_cpu_usage = all_info
            .common_nodes_latest_status
            .values()
            .map(|node| node.cpu)
            .sum::<f64>()
            / online_nodes_count as f64,
    );

    Ok((msg, all_info))
}
