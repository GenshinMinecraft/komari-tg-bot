use crate::{ErrorString, TelegramId};
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{FromRow, Pool, Sqlite};
use teloxide::types::Message;
use tokio::sync::OnceCell;

pub static DB_POOL: OnceCell<Pool<Sqlite>> = OnceCell::const_new();

#[derive(Debug, FromRow, Clone)]
pub struct Monitor {
    pub telegram_id: u64,
    pub monitor_url: String,
    pub notification_token: Option<String>,
}

pub async fn connect_db(sqlite_db_file: &str) -> Result<&Pool<Sqlite>, ErrorString> {
    DB_POOL
        .get_or_try_init(|| async {
            let db_url = format!("sqlite:{sqlite_db_file}");

            SqlitePoolOptions::new()
                .max_connections(5)
                .connect(&db_url)
                .await
        })
        .await
        .map_err(|e| ErrorString::from(e.to_string()))
}

pub async fn create_table(pool: &Pool<Sqlite>) -> Result<(), ErrorString> {
    // 创建表（如果不存在）
    if sqlx::query(
        "CREATE TABLE IF NOT EXISTS monitor (
             id INTEGER PRIMARY KEY,
             telegram_id INTEGER NOT NULL UNIQUE,
             monitor_url TEXT NOT NULL,
             notification_token TEXT
         )",
    )
        .execute(pool)
        .await
        .is_ok()
    {
        Ok(())
    } else {
        Err(String::from("数据库错误"))
    }
}

pub async fn query_monitor_by_telegram_id(
    pool: &Pool<Sqlite>,
    telegram_id: TelegramId,
) -> Result<Option<Monitor>, ErrorString> {
    let monitor_result = sqlx::query_as::<_, Monitor>(
        "SELECT telegram_id, monitor_url, notification_token
         FROM monitor
         WHERE telegram_id = ?",
    )
        .bind(telegram_id)
        .fetch_optional(pool)
        .await;

    if let Ok(monitor_result) = monitor_result {
        Ok(monitor_result)
    } else {
        Err(String::from("数据库错误"))
    }
}

pub async fn insert_monitor(pool: &Pool<Sqlite>, monitor: Monitor) -> Result<(), ErrorString> {
    if let Ok(Some(_)) = query_monitor_by_telegram_id(pool, monitor.telegram_id as i64).await {
        return Err(String::from("一个 Telegram 用户仅可添加一个 Komari 服务器"));
    }

    if sqlx::query(
        "INSERT OR IGNORE INTO monitor (telegram_id, monitor_url, notification_token)
         VALUES (?, ?, ?)",
    )
        .bind(monitor.telegram_id as i64)
        .bind(monitor.monitor_url)
        .bind(monitor.notification_token)
        .execute(pool)
        .await
        .is_ok()
    {
        Ok(())
    } else {
        Err(String::from("数据库错误"))
    }
}

pub async fn delete_monitor(
    pool: &Pool<Sqlite>,
    telegram_id: TelegramId,
) -> Result<(), ErrorString> {
    let _ = sqlx::query("DELETE FROM monitor WHERE telegram_id = ?")
        .bind(telegram_id)
        .execute(pool)
        .await
        .map_err(|e| ErrorString::from(e.to_string()))?;

    Ok(())
}

pub async fn update_notification_token(
    pool: &Pool<Sqlite>,
    telegram_id: TelegramId,
    token: String,
) -> Result<(), ErrorString> {
    let result = sqlx::query("UPDATE monitor SET notification_token = ? WHERE telegram_id = ?")
        .bind(&token)
        .bind(telegram_id)
        .execute(pool)
        .await;

    match result {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("更新 notification_token 失败: {e}")),
    }
}

pub async fn get_telegram_id(msg: &Message) -> Result<TelegramId, ErrorString> {
    let telegram_id = if let Some(user) = msg.from.clone() {
        user.id.0 as i64
    } else {
        return Err(String::from("无法获取用户ID"));
    };

    Ok(telegram_id)
}
