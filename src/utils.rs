use std::fmt::Formatter;
use crate::MessageString;
use serde::{Deserialize, Serialize};

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

fn mask_url(text: &str) -> String {
    use regex::Regex;

    let url_regex = Regex::new(r"https?://[^\s]+").unwrap();

    url_regex.replace_all(text, |caps: &regex::Captures| {
        let url = &caps[0];
        if let Some(protocol_end) = url.find("://") {
            let protocol = &url[..protocol_end + 3];
            let rest = &url[protocol_end + 3..];
            if rest.len() > 13 {
                format!("{}{}***{}", protocol, &rest[..10], &rest[rest.len()-3..])
            } else {
                format!("{}***", protocol)
            }
        } else {
            "***".to_string()
        }
    }).to_string()
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Config {
    pub db_file: String,
    pub telegram_token: String,
    pub bot_name: String,
    pub callback_http_listen: String,
    pub callback_http_url: String,
    pub log_level: String,
    pub admin_id: i64,
}

pub type ErrorString = String;

pub enum ErrorType {
    UserNotConnected,
    DataBaseError { error: ErrorString },
    UnableToParseCommand,
    EnvironmentVariablesUndefined { var: String },
    UnableToCreateReqwestClient { error: ErrorString },
    RequestError { error: ErrorString },
    JsonParseError { error: ErrorString },
    UnableToFindServerByUUID,
    GeneralError { error: ErrorString },
}

impl std::fmt::Display for ErrorType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorType::UserNotConnected => {
                write!(f, "未连接 Komari，请使用 /connect [KOMARI_HTTP_URL] 连接")
            }
            ErrorType::DataBaseError { error } => {
                write!(f, "数据库错误: {}", error)
            }
            ErrorType::UnableToParseCommand => {
                write!(f, "无法解析命令")
            }
            ErrorType::EnvironmentVariablesUndefined { var } => {
                write!(f, "环境变量未定义: {}", var)
            }
            ErrorType::UnableToCreateReqwestClient { error } => {
                write!(f, "无法创建 Reqwest 客户端: {}", error)
            }
            ErrorType::RequestError { error } => {
                let masked_error = mask_url(error);
                write!(f, "请求错误: {}", masked_error)
            }
            ErrorType::JsonParseError { error } => {
                write!(f, "JSON 解析错误: {}", error)
            }
            ErrorType::UnableToFindServerByUUID => {
                write!(f, "找不到指定 UUID 的服务器，请检查是否在 Komari 后台新建机器后，未连接上报导致无数据")
            }
            ErrorType::GeneralError { error } => {
                write!(f, "发生错误: {}", error)
            }
        }
    }
}