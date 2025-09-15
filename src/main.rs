#![warn(clippy::all, clippy::pedantic)]

mod db;
mod http_webhook;
mod json_rpc;

use crate::db::get_telegram_id;
use crate::http_webhook::generate_notification_token;
use crate::json_rpc::connect::{connect_komari_with_update_db, update_connection};
use crate::json_rpc::get_node_id::get_node_id_list;
use crate::json_rpc::status::{get_node_id_by_name, make_keyboard_for_single, status_with_id};
use crate::json_rpc::total_status::total_status;
use db::{connect_db, create_table, delete_monitor, DB_POOL};
use log::info;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::{env, fs};
use teloxide::prelude::*;
use teloxide::sugar::bot::BotMessagesExt;
use teloxide::sugar::request::RequestLinkPreviewExt;
use teloxide::types::{ParseMode, ReplyParameters};
use teloxide::utils::command::parse_command;

pub type ErrorString = String;
pub type MessageString = String; // With formated but did not escape
pub type TelegramId = i64;

#[must_use]
pub fn msg_fixer(msg: MessageString) -> String {
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

#[derive(Deserialize, Serialize, Clone)]
struct Config {
    db_file: String,
    telegram_token: String,
    bot_name: String,
    callback_http_listen: String,
    callback_http_url: String,
    log_level: String,
    admin_id: i64,
}

#[tokio::main]
async fn main() {
    let config_file = String::from_utf8(fs::read("config.json").unwrap()).unwrap();
    let config: Config = serde_json::from_str(config_file.as_str()).unwrap();

    simple_logger::init_with_level(match config.log_level.as_str() {
        "debug" => log::Level::Debug,
        "info" => log::Level::Info,
        "warn" => log::Level::Warn,
        "error" => log::Level::Error,
        _ => log::Level::Info,
    })
        .unwrap();

    unsafe {
        env::set_var("TG_TOKEN", config.telegram_token.clone());
        env::set_var("CALLBACK_HTTP_LISTEN", config.callback_http_listen);
        env::set_var("CALLBACK_HTTP_URL", config.callback_http_url.clone());
        env::set_var("BOT_NAME", config.bot_name.clone());
        env::set_var("ADMIN_ID", config.admin_id.to_string());
    };

    info!("Starting...");
    let bot = Bot::new(config.telegram_token);

    match connect_db(config.db_file.as_str()).await {
        Ok(pool) => match create_table(pool).await {
            Ok(()) => info!("数据库已创建表 / 创建表成功"),
            Err(e) => log::error!("数据库创建表失败: {e}"),
        },
        Err(e) => log::error!("连接数据库失败: {e}"),
    }

    tokio::spawn(http_webhook::start_server(
        |param1, param2, param3, body| {
            Box::pin(http_webhook::http_callback(param1, param2, param3, body))
        },
    ));

    let handler = dptree::entry()
        .branch(
            Update::filter_message().endpoint(move |bot: Bot, msg: Message| async move {
                let Ok(bot_name) = env::var("BOT_NAME") else {
                    log::error!("BOT_NAME 未设置");
                    return Ok(());
                };

                let command = match parse(msg.text().unwrap_or(""), bot_name.as_str()) {
                    Ok(Some(cmd)) => {
                        info!("接收到来自 {:?} 命令: {:?}", msg.from, cmd);
                        cmd
                    }
                    _ => {
                        return Ok(());
                    }
                };
                answer(bot, msg, command).await?;
                Ok(())
            }),
        )
        .branch(Update::filter_callback_query().endpoint(callback_handler));

    Dispatcher::builder(bot, handler)
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}

#[derive(Debug)]
enum Command {
    Start,
    Help,
    Connect { http_url: String },
    Disconnect,
    Update,
    GetNodeId,
    TotalStatus,
    StatusId { node_id: i32 },
    Status { node_name: String },
    GenerateNotificationToken,
}

fn parse(text: &str, bot_name: &str) -> Result<Option<Command>, ErrorString> {
    if !text.starts_with('/') {
        return Ok(None);
    }

    let (cmd, args) = match parse_command(text, bot_name).ok_or("无法解析的命令") {
        Ok(cmd) => cmd,
        Err(_) => return Ok(None),
    };

    match cmd {
        "start" => Ok(Some(Command::Start)),
        "help" => Ok(Some(Command::Help)),
        "connect" => {
            let http_url = args.first().ok_or("缺少HTTP URL")?;

            let http_url = if http_url.ends_with('/') {
                http_url.trim_end_matches('/')
            } else {
                http_url
            };

            Ok(Some(Command::Connect {
                http_url: http_url.to_string(),
            }))
        }
        "disconnect" => Ok(Some(Command::Disconnect)),
        "update" => Ok(Some(Command::Update)),
        "get_node_id" => Ok(Some(Command::GetNodeId)),
        "total_status" => Ok(Some(Command::TotalStatus)),
        "status" => match args.first() {
            None => Ok(Some(Command::StatusId { node_id: 1 })),
            Some(node_name) => Ok(Some(Command::Status {
                node_name: (*node_name).to_string(),
            })),
        },
        "status_id" => {
            let node_id = args.first().unwrap_or(&"1").parse::<i32>().unwrap_or(1);
            Ok(Some(Command::StatusId { node_id }))
        }
        "generate_notification_token" => Ok(Some(Command::GenerateNotificationToken)),
        _ => Ok(None),
    }
}

async fn answer(bot: Bot, msg: Message, cmd: Command) -> ResponseResult<()> {
    let telegram_id = match get_telegram_id(&msg) {
        Ok(tg_id) => tg_id,
        Err(_) => return Ok(()),
    };

    if msg
        .clone()
        .from
        .is_none_or(|user| user.is_channel())
    {
        return Ok(());
    }

    match cmd {
        Command::Start => {
            bot.send_message(
                msg.chat.id,
                r"欢迎使用 Komari Unofficial Telegram Bot

输入 /help 查看使用方法

> 本 Bot 开源于 [Github](https://github.com/GenshinMinecraft/komari-tg-bot), 使用强力的 [Rust](https://www.rust-lang.org/) 驱动, 爱来自 [Komari](https://github.com/komari-monitor/komari)", )
                .reply_parameters(ReplyParameters::new(msg.id))
                .parse_mode(ParseMode::MarkdownV2)
                .disable_link_preview(true)
                .await?;

            Ok(())
        }
        Command::Help => {
            bot.send_message(
                msg.chat.id,
                r"Komari Unofficial Telegram Bot
/start, /help - 打印本菜单

/connect HTTP_URL - 连接到 Komari 服务 (自动推断 WebSocket URL)
/disconnect - 断开已保存的连接
/update - 更新已保存的连接 (增删服务器或疑难杂症可使用)

/total_status - 获取所有节点的运行状态
/status NODE_NAME - 获取指定节点的运行状态 (第一个包含 NODE_NAME 字符串的节点，若未传入则等同于 /status_id 1)
/get_node_id - 获取所有节点的 ID (仅本 Bot)
/status_id NODE_ID - 获取指定节点 ID (使用 /get_node_id 获取节点的 ID) 的运行状态

/generate_notification_token - 生成通知令牌
",
            )
                .reply_parameters(ReplyParameters::new(msg.id))
                .disable_link_preview(true)
                .await?;
            Ok(())
        }
        Command::Connect { http_url } => {
            let url = match Url::parse(&http_url) {
                Ok(url) => url,
                Err(e) => {
                    bot.send_message(msg.chat.id, format!("无效的 URL: {e}"))
                        .reply_parameters(ReplyParameters::new(msg.id))
                        .await?;
                    return Ok(());
                }
            };

            let host = match url.host_str() {
                None => {
                    bot.send_message(msg.chat.id, "无效的 URL")
                        .reply_parameters(ReplyParameters::new(msg.id))
                        .await?;
                    return Ok(());
                }
                Some(host) => host,
            };

            let port = match url.port() {
                None => String::new(),
                Some(port) => format!(":{port}"),
            };

            let http_url = format!("{}://{}{}", url.scheme(), host, port);

            match connect_komari_with_update_db(http_url, telegram_id).await {
                Ok(message) => {
                    bot.send_message(msg.chat.id, msg_fixer(message))
                        .parse_mode(ParseMode::MarkdownV2)
                        .reply_parameters(ReplyParameters::new(msg.id))
                        .await?;
                }
                Err(e) => {
                    bot.send_message(msg.chat.id, format!("获取站点信息失败: {e}"))
                        .reply_parameters(ReplyParameters::new(msg.id))
                        .await?;
                }
            }
            Ok(())
        }
        Command::Disconnect => {
            let db_pool = DB_POOL
                .get()
                .unwrap_or_else(|| panic!("数据库连接池未初始化"));

            match delete_monitor(db_pool, telegram_id).await {
                Ok(()) => {
                    bot.send_message(msg.chat.id, "已取消连接到 Komari")
                        .reply_parameters(ReplyParameters::new(msg.id))
                        .await?;
                    Ok(())
                }
                Err(e) => {
                    bot.send_message(msg.chat.id, format!("取消连接到 Komari 失败: {e}"))
                        .reply_parameters(ReplyParameters::new(msg.id))
                        .await?;
                    Ok(())
                }
            }
        }
        Command::Update => {
            match update_connection(telegram_id).await {
                Ok(message) => {
                    bot.send_message(msg.chat.id, msg_fixer(message))
                        .parse_mode(ParseMode::MarkdownV2)
                        .reply_parameters(ReplyParameters::new(msg.id))
                        .await?;
                }
                Err(e) => {
                    bot.send_message(msg.chat.id, format!("获取站点信息失败: {e}"))
                        .reply_parameters(ReplyParameters::new(msg.id))
                        .await?;
                }
            }

            Ok(())
        }
        Command::GetNodeId => match get_node_id_list(telegram_id).await {
            Ok((message, _, _)) => {
                bot.send_message(msg.chat.id, msg_fixer(message))
                    .parse_mode(ParseMode::MarkdownV2)
                    .reply_parameters(ReplyParameters::new(msg.id))
                    .await?;
                Ok(())
            }
            Err(e) => {
                bot.send_message(msg.chat.id, format!("无法获取节点ID: {e}"))
                    .reply_parameters(ReplyParameters::new(msg.id))
                    .await?;
                Ok(())
            }
        },
        Command::TotalStatus => {
            let message_str = match total_status(telegram_id).await {
                Ok(message_str) => message_str.0,
                Err(e) => {
                    bot.send_message(msg.chat.id, format!("无法解析 Komari Websocket 数据: {e}"))
                        .reply_parameters(ReplyParameters::new(msg.id))
                        .await?;
                    return Ok(());
                }
            };

            bot.send_message(msg.chat.id, msg_fixer(message_str))
                .parse_mode(ParseMode::MarkdownV2)
                .reply_parameters(ReplyParameters::new(msg.id))
                .disable_link_preview(true)
                .await?;

            Ok(())
        }
        Command::Status { node_name } => {
            let (msg_str, all_info, node_id) =
                match get_node_id_by_name(telegram_id, node_name).await {
                    Ok(msg) => msg,
                    Err(e) => {
                        bot.send_message(msg.chat.id, format!("无法解析 Komari 数据: {e}"))
                            .reply_parameters(ReplyParameters::new(msg.id))
                            .await?;
                        return Ok(());
                    }
                };

            let keyboard = match make_keyboard_for_single(node_id, telegram_id, &all_info).await {
                Ok(key) => key,
                Err(e) => {
                    bot.send_message(msg.chat.id, format!("无法生成键盘: {e}"))
                        .reply_parameters(ReplyParameters::new(msg.id))
                        .await?;
                    return Ok(());
                }
            };

            bot.send_message(msg.chat.id, msg_fixer(msg_str))
                .parse_mode(ParseMode::MarkdownV2)
                .reply_parameters(ReplyParameters::new(msg.id))
                .reply_markup(keyboard)
                .disable_link_preview(true)
                .await?;

            Ok(())
        }
        Command::StatusId { node_id } => {
            let (msg_str, all_info) = match status_with_id(telegram_id, node_id as u32).await {
                Ok(msg) => msg,
                Err(e) => {
                    bot.send_message(msg.chat.id, format!("无法解析 Komari 数据: {e}"))
                        .reply_parameters(ReplyParameters::new(msg.id))
                        .await?;
                    return Ok(());
                }
            };

            let keyboard = match make_keyboard_for_single(node_id, telegram_id, &all_info).await {
                Ok(key) => key,
                Err(e) => {
                    bot.send_message(msg.chat.id, format!("无法生成键盘: {e}"))
                        .reply_parameters(ReplyParameters::new(msg.id))
                        .await?;
                    return Ok(());
                }
            };

            bot.send_message(msg.chat.id, msg_fixer(msg_str))
                .parse_mode(ParseMode::MarkdownV2)
                .reply_parameters(ReplyParameters::new(msg.id))
                .reply_markup(keyboard)
                .disable_link_preview(true)
                .await?;

            Ok(())
        }
        Command::GenerateNotificationToken => {
            if !msg.chat.is_private() {
                bot.send_message(msg.chat.id, "此命令只能用于私聊")
                    .reply_parameters(ReplyParameters::new(msg.id))
                    .await?;
                return Ok(());
            }

            match generate_notification_token(telegram_id).await {
                Ok(message) => {
                    bot.send_message(msg.chat.id, msg_fixer(message))
                        .parse_mode(ParseMode::MarkdownV2)
                        .reply_parameters(ReplyParameters::new(msg.id))
                        .await?;
                }
                Err(e) => {
                    bot.send_message(msg.chat.id, format!("无法生成通知令牌: {e}"))
                        .reply_parameters(ReplyParameters::new(msg.id))
                        .await?;
                }
            }

            Ok(())
        }
    }
}

async fn callback_handler(bot: Bot, q: CallbackQuery) -> Result<(), Box<dyn Error + Send + Sync>> {
    if let Some(ref node_id) = q.data {
        bot.answer_callback_query(q.id.clone()).await?;

        let (callback_tg_id, node_id) = {
            let split: Vec<String> = node_id
                .split('-')
                .map(std::string::ToString::to_string)
                .collect();
            (
                split
                    .first()
                    .ok_or("Invalid callback data".to_string())?
                    .clone(),
                split
                    .get(1)
                    .ok_or("Invalid callback data".to_string())?
                    .clone(),
            )
        };

        let telegram_id = callback_tg_id
            .parse::<i64>()
            .map_err(|_| "Invalid callback data".to_string())?;
        let node_id = node_id
            .parse::<i32>()
            .map_err(|_| "Invalid callback data".to_string())?;

        if telegram_id != q.from.id.0 as i64 {
            return Ok(());
        }

        let (msg_str, all_info) = match status_with_id(telegram_id, node_id as u32).await {
            Ok(msg) => msg,
            Err(e) => {
                if let Some(message) = q.regular_message() {
                    bot.edit_text(message, format!("无法解析 Komari 数据: {e}"))
                        .parse_mode(ParseMode::MarkdownV2)
                        .disable_link_preview(true)
                        .await?;
                } else if let Some(id) = q.inline_message_id {
                    bot.edit_message_text_inline(id, format!("无法解析 Komari 数据: {e}"))
                        .parse_mode(ParseMode::MarkdownV2)
                        .await?;
                }
                return Ok(());
            }
        };

        let keyboard = match make_keyboard_for_single(node_id, telegram_id, &all_info).await {
            Ok(key) => key,
            Err(e) => {
                if let Some(message) = q.regular_message() {
                    bot.edit_text(message, format!("无法生成键盘: {e}"))
                        .parse_mode(ParseMode::MarkdownV2)
                        .disable_link_preview(true)
                        .await?;
                } else if let Some(id) = q.inline_message_id {
                    bot.edit_message_text_inline(id, format!("无法生成键盘: {e}"))
                        .parse_mode(ParseMode::MarkdownV2)
                        .await?;
                }
                return Ok(());
            }
        };

        if let Some(message) = q.regular_message() {
            bot.edit_text(message, msg_fixer(msg_str))
                .reply_markup(keyboard)
                .parse_mode(ParseMode::MarkdownV2)
                .disable_link_preview(true)
                .await?;
        } else if let Some(id) = q.inline_message_id {
            bot.edit_message_text_inline(id, msg_fixer(msg_str))
                .reply_markup(keyboard)
                .parse_mode(ParseMode::MarkdownV2)
                .await?;
        }
    }

    Ok(())
}
