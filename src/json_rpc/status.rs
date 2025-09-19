use crate::json_rpc::bytes_to_pretty_string;
use crate::json_rpc::get_node_id::get_node_id_list;
use crate::json_rpc::query::AllInfo;
use crate::{ErrorString, MessageString, TelegramId};
use reqwest::Url;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

pub async fn status_with_id(
    telegram_id: TelegramId,
    index: u32,
) -> Result<(MessageString, AllInfo), ErrorString> {
    let (_, all_info, node_id_list) = get_node_id_list(telegram_id).await?;

    let vec_index: usize = match index {
        0 | 1 => 0,
        _ => (index - 1) as usize,
    };

    let (node_uuid, node_latest_info) = node_id_list
        .get(vec_index)
        .ok_or(ErrorString::from("节点不存在"))?;

    let node_info = all_info
        .common_nodes
        .values()
        .find(|n| n.uuid == *node_uuid)
        .cloned()
        .ok_or(ErrorString::from("无法找到该服务器"))?;

    let (ram_used, ram_total, ram_usage) = {
        let ram_used = node_latest_info.ram;
        let ram_total = node_latest_info.ram_total;
        let ram_usage = (ram_used as f64 / ram_total as f64) * 100.0;
        (
            bytes_to_pretty_string(ram_used),
            bytes_to_pretty_string(ram_total),
            ram_usage,
        )
    };

    let (swap_used, swap_total, swap_usage) = {
        let swap_used = node_latest_info.swap;
        let swap_total = node_latest_info.swap_total;
        let swap_usage = (swap_used as f64 / swap_total as f64) * 100.0;
        (
            bytes_to_pretty_string(swap_used),
            bytes_to_pretty_string(swap_total),
            swap_usage,
        )
    };

    let (disk_used, disk_total, disk_usage) = {
        let disk_used = node_latest_info.disk;
        let disk_total = node_latest_info.disk_total;
        let disk_usage = (disk_used as f64 / disk_total as f64) * 100.0;
        (
            bytes_to_pretty_string(disk_used),
            bytes_to_pretty_string(disk_total),
            disk_usage,
        )
    };

    let msg = format!(
        r"{title} | {region} | {name}

CPU: `{cpu_name}` @ `{cpu_cores} Cores`{gpu_name}
ARCH: `{arch}`
VIRT: `{virtualization}`
OS: `{os}`
KERN: `{kernel_version}`
UPTIME: `{uptime}`

CPU: `{cpu_usage:.2}%`
RAM: `{ram_used}` / `{ram_total}` `{ram_usage:.2}%`
SWAP: `{swap_used}` / `{swap_total}` `{swap_usage:.2}%`
DISK: `{disk_used}` / `{disk_total}` `{disk_usage:.2}%`

LOAD: `{load1:.2}` / `{load5:.2}` / `{load15:.2}`
PROC: `{processes}`

NET: `{total_net_down}` / `{total_net_up}`
UP: `{net_up:.2} Mbps`
DOWN: `{net_down:.2} Mbps`
CONN: `{total_tcp_connections} TCP` / `{total_udp_connections} UDP`{update_at}",
        title = all_info.common_public_info.sitename,
        region = node_info.region,
        name = node_info.name,
        arch = node_info.arch,
        cpu_name = node_info.cpu_name,
        cpu_cores = node_info.cpu_cores,
        virtualization = node_info.virtualization,
        os = node_info.os,
        kernel_version = node_info.kernel_version,
        uptime = 0,
        cpu_usage = node_latest_info.cpu,
        load1 = node_latest_info.load,
        load5 = node_latest_info.load5,
        load15 = node_latest_info.load15,
        processes = node_latest_info.process,
        total_net_down = bytes_to_pretty_string(node_latest_info.net_total_down),
        total_net_up = bytes_to_pretty_string(node_latest_info.net_total_up),
        net_down = node_latest_info.net_in as f64 / 125000.0,
        net_up = node_latest_info.net_out as f64 / 125000.0,
        total_tcp_connections = node_latest_info.connections,
        total_udp_connections = node_latest_info.connections_udp,
        update_at = {
            if let Some(updated_at) = node_info.updated_at {
                format!(
                    "

UPDATE AT: `{updated_at}`"
                )
            } else {
                String::new()
            }
        },
        gpu_name = {
            if node_info.gpu_name.is_empty() {
                String::new()
            } else {
                format!(
                    "
GPU: `{}`",
                    node_info.gpu_name
                )
            }
        }
    );

    Ok((msg, all_info))
}

pub async fn get_node_id_by_name(
    telegram_id: TelegramId,
    name: String,
) -> Result<(MessageString, AllInfo, i32), ErrorString> {
    let (message_str, _, _) = get_node_id_list(telegram_id).await?;

    let mut selected_node_id = -1;
    for line in message_str.lines() {
        let (node_id, node_name) = line
            .split_once(" - ")
            .ok_or(String::from("无法解析节点信息"))?;

        selected_node_id = if node_name.contains(name.as_str()) {
            node_id
                .trim_matches('`')
                .parse::<i32>()
                .map_err(|_| "无法解析节点ID")?
        } else {
            continue;
        };
    }

    let (msg, all_info) = status_with_id(telegram_id, selected_node_id as u32).await?;

    Ok((msg, all_info, selected_node_id))
}

pub async fn make_keyboard_for_single(
    now_id: i32,
    telegram_id: i64,
    all_info: &AllInfo,
) -> Result<InlineKeyboardMarkup, ErrorString> {
    let max_server = all_info.common_nodes.iter().len();

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
        format!("{telegram_id}-{now_id}"),
    )]);

    Ok(InlineKeyboardMarkup::new(keyboard))
}
