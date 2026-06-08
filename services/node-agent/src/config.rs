use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use anyhow::{Context, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    #[serde(rename = "nodeId")]
    pub node_id: String,
    #[serde(rename = "nodeName")]
    pub node_name: String,
    #[serde(rename = "ownerDiscordUserId")]
    pub owner_discord_user_id: String,
    #[serde(rename = "managerUrl")]
    pub manager_url: String,
    #[serde(rename = "nodeToken")]
    pub node_token: String,
    #[serde(rename = "clusterSharePath")]
    pub cluster_share_path: String,
    #[serde(rename = "smbUncPath", default)]
    pub smb_unc_path: String,
    #[serde(rename = "smbUser", default)]
    pub smb_user: String,
    #[serde(rename = "smbPassword", default)]
    pub smb_password: String,
    #[serde(rename = "arkDedicatedDir")]
    pub ark_dedicated_dir: String,
    #[serde(rename = "modIds")]
    pub mod_ids: Vec<u64>,
    #[serde(rename = "gamePort")]
    pub game_port: u16,
    #[serde(rename = "rawPort")]
    pub raw_port: u16,
    #[serde(rename = "queryPort")]
    pub query_port: u16,
    #[serde(rename = "rconPort")]
    pub rcon_port: u16,
    #[serde(rename = "serverAdminPassword")]
    pub server_admin_password: String,
    #[serde(rename = "logDir")]
    pub log_dir: String,
    #[serde(rename = "backupDir")]
    pub backup_dir: String,
    #[serde(rename = "heartbeatIntervalSecs", default = "default_heartbeat")]
    pub heartbeat_interval_secs: u64,
    #[serde(rename = "taskPollIntervalSecs", default = "default_task_poll")]
    pub task_poll_interval_secs: u64,
    #[serde(rename = "maxRamMbBeforeBlock", default = "default_max_ram")]
    pub max_ram_mb_before_block: u64,
}

fn default_heartbeat() -> u64 { 30 }
fn default_task_poll() -> u64 { 5 }
fn default_max_ram() -> u64 { 3072 }

impl NodeConfig {
    pub fn load(path: &str) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("cannot read config: {path}"))?;
        serde_json::from_str(&content)
            .with_context(|| format!("cannot parse config: {path}"))
    }

    pub fn ark_server_exe(&self) -> PathBuf {
        PathBuf::from(&self.ark_dedicated_dir)
            .join("ShooterGame")
            .join("Binaries")
            .join("Win64")
            .join("ShooterGameServer.exe")
    }

    pub fn mods_dir(&self) -> PathBuf {
        PathBuf::from(&self.ark_dedicated_dir)
            .join("ShooterGame")
            .join("Content")
            .join("Mods")
    }

    pub fn validate_minimal(&self) -> Result<()> {
        if self.node_id.is_empty() { anyhow::bail!("nodeId is required"); }
        if self.node_token.is_empty() { anyhow::bail!("nodeToken is required — run setup.ps1 first"); }
        if self.manager_url.is_empty() { anyhow::bail!("managerUrl is required"); }
        Ok(())
    }
}

pub fn default_config_path() -> String {
    r"C:\ProgramData\ArkClusterNode\config.json".to_string()
}
