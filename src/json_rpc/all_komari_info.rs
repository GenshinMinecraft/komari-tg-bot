use crate::db::{DB_POOL, get_all_monitors};
use crate::json_rpc::query::{AllInfo, get_all_info};
use crate::{ErrorString, MessageString};
use log::error;
use tokio::sync::mpsc;

pub async fn get_every_one_status() -> Result<MessageString, ErrorString> {
    let db = DB_POOL.get().ok_or(String::from("无法获取数据库"))?;

    let monitors = get_all_monitors(db).await?;

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

    let cpu_cores = all_infos
        .iter()
        .map(|all_info| {
            all_info
                .common_nodes
                .iter()
                .map(|node| node.1.cpu_cores)
                .sum::<i64>()
        })
        .sum::<i64>();

    return Ok(format!("{cpu_cores}"));

    todo!()
}
