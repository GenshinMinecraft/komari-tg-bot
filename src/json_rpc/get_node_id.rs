use crate::db::{query_monitor_by_telegram_id, DB_POOL};
use crate::json_rpc::query::{AllInfo, CommonGetNodesLatestStatusSingle};
use crate::{ErrorString, MessageString, TelegramId};

type NodeUuid = String;
pub type SortedNodeList = Vec<(NodeUuid, CommonGetNodesLatestStatusSingle)>;

pub async fn get_node_id_list(
    telegram_id: TelegramId,
) -> Result<(MessageString, AllInfo, SortedNodeList), ErrorString> {
    let db = DB_POOL.get().ok_or(String::from("无法获取数据库"))?;

    let Some(monitor) = query_monitor_by_telegram_id(db, telegram_id).await? else {
        return Err(ErrorString::from(
            "服务器未连接，请先使用 /connect [http url] 连接".to_string(),
        ));
    };

    let all_info = crate::json_rpc::query::get_all_info(&monitor.monitor_url).await?;

    let mut node_list = all_info
        .common_nodes_latest_status
        .iter()
        .map(|(node_uuid, node_name)| (node_uuid.clone(), node_name.clone()))
        .collect::<Vec<_>>();
    node_list.sort_by(|a, b| a.0.cmp(&b.0));

    let mut message_str = String::new();
    {
        let mut counter = 0;

        for (node_uuid, _) in node_list.clone() {
            counter += 1;

            // 查找uuid相符的节点并获取其名称
            let node_name = all_info
                .common_nodes
                .values()
                .find(|n| n.uuid == node_uuid)
                .map(|n| n.name.clone())
                .unwrap_or_else(|| "未知节点".to_string());

            message_str.push_str(&format!("`{}` - {}\n", counter, node_name));
        }
    }

    Ok((message_str, all_info, node_list))
}
