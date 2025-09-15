use crate::json_rpc::create_reqwest_client;
use crate::ErrorString;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonRpcRequestBase {
    pub jsonrpc: String,
    pub method: String,
    pub id: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonRpcResponseBase {
    pub jsonrpc: String,
    pub id: i64,
    pub result: Value,
}

const JSON_RPC_VERSION: &str = "2.0";
const JSON_RPC_METHOD: [&str; 9] = [
    "rpc.help",
    "rpc.methods",
    "rpc.ping",
    "rpc.version",
    "common:getPublicInfo",
    "common:getNodes",
    "common:getNodesLatestStatus",
    "common:getMe",
    "common:getVersion",
];

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AllInfo {
    pub rpc_help: RpcHelp,
    pub rpc_methods: RpcMethods,
    pub rpc_ping: RpcPing,
    pub rpc_version: RpcVersion,
    pub common_public_info: CommonGetPublicInfo,
    pub common_nodes: CommonGetNodes,
    pub common_nodes_latest_status: CommonGetNodesLatestStatus,
    pub common_me: CommonGetMe,
    pub common_version: CommonGetVersion,
}

pub async fn get_all_info(http_url: &str) -> Result<AllInfo, ErrorString> {
    let client = create_reqwest_client().await?;

    let url = format!("{http_url}/api/rpc2");

    let json_rpc_post_body = JSON_RPC_METHOD
        .iter()
        .enumerate()
        .map(|(index, method)| JsonRpcRequestBase {
            jsonrpc: JSON_RPC_VERSION.to_string(),
            method: (*method).to_string(),
            id: (index + 1) as i64,
        })
        .collect::<Vec<_>>();

    let response = client
        .post(&url)
        .json(&json_rpc_post_body)
        .send()
        .await
        .map_err(|e| ErrorString::from(format!("请求错误: {e}")))?;

    let json_rpc_response_body = response
        .json::<Vec<JsonRpcResponseBase>>()
        .await
        .map_err(|e| ErrorString::from(format!("解析错误: {e}")))?;

    let rpc_help: RpcHelp = serde_json::from_value(
        json_rpc_response_body
            .iter()
            .find(|response| response.id == 1)
            .ok_or_else(|| ErrorString::from("Json 解析错误: 未找到 id 为 1 的响应"))?
            .result
            .clone(),
    )
        .map_err(|e| format!("Json 解析错误: 未找到 id 为 1 的响应: {e}"))?;

    let rpc_methods: RpcMethods = serde_json::from_value(
        json_rpc_response_body
            .iter()
            .find(|response| response.id == 2)
            .ok_or_else(|| ErrorString::from("Json 解析错误: 未找到 id 为 2 的响应"))?
            .result
            .clone(),
    )
        .map_err(|e| format!("Json 解析错误: 未找到 id 为 2 的响应: {e}"))?;

    let rpc_ping: RpcPing = serde_json::from_value(
        json_rpc_response_body
            .iter()
            .find(|response| response.id == 3)
            .ok_or_else(|| ErrorString::from("Json 解析错误: 未找到 id 为 3 的响应"))?
            .result
            .clone(),
    )
        .map_err(|e| format!("Json 解析错误: 未找到 id 为 3 的响应: {e}"))?;

    let rpc_version: RpcVersion = serde_json::from_value(
        json_rpc_response_body
            .iter()
            .find(|response| response.id == 4)
            .ok_or_else(|| ErrorString::from("Json 解析错误: 未找到 id 为 4 的响应"))?
            .result
            .clone(),
    )
        .map_err(|e| format!("Json 解析错误: 未找到 id 为 4 的响应: {e}"))?;

    let common_get_public_info: CommonGetPublicInfo = serde_json::from_value(
        json_rpc_response_body
            .iter()
            .find(|response| response.id == 5)
            .ok_or_else(|| ErrorString::from("Json 解析错误: 未找到 id 为 5 的响应"))?
            .result
            .clone(),
    )
        .map_err(|e| format!("Json 解析错误: 未找到 id 为 5 的响应: {e}"))?;

    let common_get_nodes: CommonGetNodes = serde_json::from_value(
        json_rpc_response_body
            .iter()
            .find(|response| response.id == 6)
            .ok_or_else(|| ErrorString::from("Json 解析错误: 未找到 id 为 6 的响应"))?
            .result
            .clone(),
    )
        .map_err(|e| format!("Json 解析错误: 未找到 id 为 6 的响应: {e}"))?;

    let common_get_nodes_latest_status: CommonGetNodesLatestStatus = serde_json::from_value(
        json_rpc_response_body
            .iter()
            .find(|response| response.id == 7)
            .ok_or_else(|| ErrorString::from("Json 解析错误: 未找到 id 为 7 的响应"))?
            .result
            .clone(),
    )
        .map_err(|e| format!("Json 解析错误: 未找到 id 为 7 的响应: {e}"))?;

    let common_get_me: CommonGetMe = serde_json::from_value(
        json_rpc_response_body
            .iter()
            .find(|response| response.id == 8)
            .ok_or_else(|| ErrorString::from("Json 解析错误: 未找到 id 为 8 的响应"))?
            .result
            .clone(),
    )
        .map_err(|e| format!("Json 解析错误: 未找到 id 为 8 的响应: {e}"))?;

    let common_get_version: CommonGetVersion = serde_json::from_value(
        json_rpc_response_body
            .iter()
            .find(|response| response.id == 9)
            .ok_or_else(|| ErrorString::from("Json 解析错误: 未找到 id 为 9 的响应"))?
            .result
            .clone(),
    )
        .map_err(|e| format!("Json 解析错误: 未找到 id 为 9 的响应: {e}"))?;

    Ok(AllInfo {
        rpc_help,
        rpc_methods,
        rpc_ping,
        rpc_version,
        common_public_info: common_get_public_info,
        common_nodes: common_get_nodes,
        common_nodes_latest_status: common_get_nodes_latest_status,
        common_me: common_get_me,
        common_version: common_get_version,
    })
}

pub type RpcHelp = Vec<RpcHelpSingle>;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcHelpSingle {
    pub name: String,
    pub summary: String,
}

pub type RpcMethods = Vec<String>;

pub type RpcPing = String;

pub type RpcVersion = String;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommonGetPublicInfo {
    #[serde(rename = "allow_cors")]
    pub allow_cors: bool,
    #[serde(rename = "custom_body")]
    pub custom_body: String,
    #[serde(rename = "custom_head")]
    pub custom_head: String,
    pub description: String,
    #[serde(rename = "disable_password_login")]
    pub disable_password_login: bool,
    #[serde(rename = "oauth_enable")]
    pub oauth_enable: bool,
    #[serde(rename = "oauth_provider")]
    pub oauth_provider: String,
    #[serde(rename = "ping_record_preserve_time")]
    pub ping_record_preserve_time: i64,
    #[serde(rename = "private_site")]
    pub private_site: bool,
    #[serde(rename = "record_enabled")]
    pub record_enabled: bool,
    #[serde(rename = "record_preserve_time")]
    pub record_preserve_time: i64,
    pub sitename: String,
    pub theme: String,
}

pub type NodeUuid = String;
pub type CommonGetNodes = HashMap<NodeUuid, CommonGetNodesSingle>;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommonGetNodesSingle {
    pub uuid: String,
    pub name: String,
    #[serde(rename = "cpu_name")]
    pub cpu_name: String,
    pub virtualization: String,
    pub arch: String,
    #[serde(rename = "cpu_cores")]
    pub cpu_cores: i64,
    pub os: String,
    #[serde(rename = "kernel_version")]
    pub kernel_version: String,
    #[serde(rename = "gpu_name")]
    pub gpu_name: String,
    pub region: String,
    #[serde(rename = "mem_total")]
    pub mem_total: i64,
    #[serde(rename = "swap_total")]
    pub swap_total: i64,
    #[serde(rename = "disk_total")]
    pub disk_total: i64,
    pub group: Option<String>,
    pub tags: Option<String>,
    #[serde(rename = "created_at")]
    pub created_at: Option<String>,
    #[serde(rename = "updated_at")]
    pub updated_at: Option<String>,
}

pub type CommonGetNodesLatestStatus = HashMap<NodeUuid, CommonGetNodesLatestStatusSingle>;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommonGetNodesLatestStatusSingle {
    pub client: String,
    pub time: String,
    pub cpu: f64,
    pub gpu: f64,
    pub ram: i64,
    #[serde(rename = "ram_total")]
    pub ram_total: i64,
    pub swap: i64,
    #[serde(rename = "swap_total")]
    pub swap_total: i64,
    pub load: f64,
    pub load5: f64,
    pub load15: f64,
    pub temp: i64,
    pub disk: i64,
    #[serde(rename = "disk_total")]
    pub disk_total: i64,
    #[serde(rename = "net_in")]
    pub net_in: i64,
    #[serde(rename = "net_out")]
    pub net_out: i64,
    #[serde(rename = "net_total_up")]
    pub net_total_up: i64,
    #[serde(rename = "net_total_down")]
    pub net_total_down: i64,
    pub process: i64,
    pub connections: i64,
    #[serde(rename = "connections_udp")]
    pub connections_udp: i64,
    pub online: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommonGetMe {
    #[serde(rename = "2fa_enabled")]
    pub n2fa_enabled: bool,
    #[serde(rename = "logged_in")]
    pub logged_in: bool,
    #[serde(rename = "sso_id")]
    pub sso_id: String,
    #[serde(rename = "sso_type")]
    pub sso_type: String,
    pub username: String,
    pub uuid: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommonGetVersion {
    pub version: String,
    pub hash: String,
}
