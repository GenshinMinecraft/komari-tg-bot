use sqlx::encode::IsNull::No;
use crate::{db, ErrorString, MessageString, TelegramId};
use crate::db::{Monitor, DB_POOL};
use crate::json_rpc::bytes_to_pretty_string;
use crate::json_rpc::query::get_all_info;

async fn connect_komari_with_update_db(http_url: String, telegram_id: TelegramId) -> Result<MessageString, ErrorString> {
    let db = DB_POOL.get().ok_or(String::from("无法获取数据库"))?;
    
    let all_info = get_all_info(&http_url).await?;
    
    let monitor = Monitor {
        telegram_id: telegram_id as u64,
        monitor_url: http_url,
        notification_token: None,
    };
    
    db::delete_monitor(db, telegram_id).await?;
    
    db::insert_monitor(db, monitor).await?;
    
    let msg: MessageString = format!(
        "成功读取 Komari 服务信息！
站点名称：`{site_name}`
站点详情：`{site_description}`
节点数量：`{nodes_count}`
CPU 核心总数：`{cores_count}`
内存总量：`{memory_total}`
交换分区总量：`{swap_total}`
硬盘总量：`{disk_total}`", site_name=all_info.common_public_info.sitename,
    site_description=all_info.common_public_info.description,
        nodes_count = all_info.common_nodes.len(),
    cores_count = all_info.common_nodes.iter().map(|node| node.1.cpu_cores).sum::<i64>(),
    memory_total = bytes_to_pretty_string(all_info.common_nodes.iter().map(|node| node.1.mem_total).sum::<i64>()),
    swap_total = bytes_to_pretty_string(all_info.common_nodes.iter().map(|node| node.1.swap_total).sum::<i64>()),
    disk_total = bytes_to_pretty_string(all_info.common_nodes.iter().map(|node| node.1.disk_total).sum::<i64>()),);
    
    todo!()
}