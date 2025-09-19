use crate::db::{DB_POOL, Monitor, get_all_monitors};
use crate::json_rpc::bytes_to_pretty_string;
use crate::json_rpc::query::{AllInfo, get_all_info};
use crate::{ErrorString, MessageString};
use log::error;
use teloxide::dptree::filter;
use tokio::sync::mpsc;

pub fn filter_valid_all_info(all_infos: Vec<AllInfo>) -> Vec<AllInfo> {
    all_infos
        .into_iter()
        .filter(|all_info| {
            all_info
                .common_nodes
                .values()
                .all(|node| node.cpu_cores <= 384)
        })
        .collect()
}

pub async fn get_every_one_status() -> Result<MessageString, ErrorString> {
    let db = DB_POOL.get().ok_or(String::from("无法获取数据库"))?;

    let monitors: Vec<Monitor> = get_all_monitors(db).await?;

    let all_user_count = monitors.len();

    let (tx, mut rx) = mpsc::channel(32);

    tokio::spawn(async move {
        for monitor in monitors {
            let tx = tx.clone();
            tokio::spawn(async move {
                let all_info = match get_all_info(monitor.monitor_url.as_str()).await {
                    Ok(all_info) => all_info,
                    Err(e) => {
                        error!("{}", e);
                        return;
                    }
                };

                if let Err(e) = tx.send(all_info).await {
                    error!("{}", e);
                }
            });
        }
        drop(tx);
    });

    let mut all_infos = vec![];
    while let Some(all_info) = rx.recv().await {
        all_infos.push(all_info);
    }

    let success_count = all_infos.len();

    let all_infos = filter_valid_all_info(all_infos);

    let (total_cpu_cores, avg_cpu_usage) = {
        let total_cpu_cores = all_infos
            .iter()
            .map(|all_info| {
                all_info
                    .common_nodes
                    .iter()
                    .map(|node| node.1.cpu_cores)
                    .sum::<i64>()
            })
            .sum::<i64>();
        let avg_cpu_usage = if total_cpu_cores > 0 {
            (all_infos
                .iter()
                .map(|all_info| all_info.common_nodes.len())
                .sum::<usize>() as f64
                / total_cpu_cores as f64)
                * 100.0
        } else {
            0.0
        };
        (total_cpu_cores, avg_cpu_usage)
    };

    let (online_nodes_count, total_nodes_count, percent_online) = {
        let online_nodes_count = all_infos
            .iter()
            .flat_map(|all_info| all_info.common_nodes_latest_status.iter())
            .filter(|(_, node)| node.online)
            .count();

        let total_nodes_count = all_infos
            .iter()
            .map(|all_info| all_info.common_nodes_latest_status.len())
            .sum::<usize>();

        let percent_online = if total_nodes_count > 0 {
            (online_nodes_count as f64 / total_nodes_count as f64) * 100.0
        } else {
            0.0
        };
        (online_nodes_count, total_nodes_count, percent_online)
    };

    let (avg_load1, avg_load5, avg_load15) = {
        let load1 = all_infos
            .iter()
            .flat_map(|all_info| all_info.common_nodes_latest_status.values())
            .map(|node| node.load)
            .sum::<f64>()
            / total_nodes_count as f64;
        let load5 = all_infos
            .iter()
            .flat_map(|all_info| all_info.common_nodes_latest_status.values())
            .map(|node| node.load5)
            .sum::<f64>()
            / total_nodes_count as f64;
        let load15 = all_infos
            .iter()
            .flat_map(|all_info| all_info.common_nodes_latest_status.values())
            .map(|node| node.load15)
            .sum::<f64>()
            / total_nodes_count as f64;
        (load1, load5, load15)
    };

    let (total_used_ram, total_total_ram, avg_ram_usage) = {
        let total_used_ram = all_infos
            .iter()
            .flat_map(|all_info| all_info.common_nodes_latest_status.values())
            .map(|node| node.ram)
            .sum::<i64>();
        let total_total_ram = all_infos
            .iter()
            .flat_map(|all_info| all_info.common_nodes_latest_status.values())
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
        let total_used_swap = all_infos
            .iter()
            .flat_map(|all_info| all_info.common_nodes_latest_status.values())
            .map(|node| node.swap)
            .sum::<i64>();
        let total_total_swap = all_infos
            .iter()
            .flat_map(|all_info| all_info.common_nodes_latest_status.values())
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
        let total_used_disk = all_infos
            .iter()
            .flat_map(|all_info| all_info.common_nodes_latest_status.values())
            .map(|node| node.disk)
            .sum::<i64>();
        let total_total_disk = all_infos
            .iter()
            .flat_map(|all_info| all_info.common_nodes_latest_status.values())
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
        let (total_total_net_down, total_total_net_up) = {
            let total_total_net_down = all_infos
                .iter()
                .flat_map(|all_info| all_info.common_nodes_latest_status.values())
                .map(|node| node.net_total_down)
                .sum::<i64>();
            let total_total_net_up = all_infos
                .iter()
                .flat_map(|all_info| all_info.common_nodes_latest_status.values())
                .map(|node| node.net_total_up)
                .sum::<i64>();
            (
                bytes_to_pretty_string(total_total_net_down),
                bytes_to_pretty_string(total_total_net_up),
            )
        };
        let (total_net_down, total_net_up) = {
            let total_net_down = all_infos
                .iter()
                .flat_map(|all_info| all_info.common_nodes_latest_status.values())
                .map(|node| node.net_in)
                .sum::<i64>();
            let total_net_up = all_infos
                .iter()
                .flat_map(|all_info| all_info.common_nodes_latest_status.values())
                .map(|node| node.net_out)
                .sum::<i64>();
            (
                total_net_down as f64 / 125000.0,
                total_net_up as f64 / 125000.0,
            )
        };
        let (total_tcp_connections, total_udp_connections) = {
            let total_tcp_connections = all_infos
                .iter()
                .flat_map(|all_info| all_info.common_nodes_latest_status.values())
                .map(|node| node.connections)
                .sum::<i64>();
            let total_udp_connections = all_infos
                .iter()
                .flat_map(|all_info| all_info.common_nodes_latest_status.values())
                .map(|node| node.connections_udp)
                .sum::<i64>();
            (total_tcp_connections, total_udp_connections)
        };
        (
            total_total_net_down,
            total_total_net_up,
            total_net_down,
            total_net_up,
            total_tcp_connections,
            total_udp_connections,
        )
    };

    let msg = format!(
        r"@komaritgbot 总览:

本 Bot 已保存连接: {all_user_count}
本 Bot 已成功读取: {success_count}

ONLINE: `{online_nodes_count}` / `{total_nodes_count}` `{percent_online:.2}%`
CPU CORES: `{total_cpu_cores}`
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
    );

    Ok(msg)
}
