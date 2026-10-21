use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// 一个 Skill 的来源信息
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SkillSourceInfo {
    /// local | github | manual
    pub kind: String,
    /// 原始路径或 URL，用于展示
    pub origin: Option<String>,
    pub owner: Option<String>,
    pub repo: Option<String>,
    pub reference: Option<String>,
    pub subpath: Option<String>,
}

/// 仓库中保存的 Skill 元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillMeta {
    /// slug，同时是部署到各 Agent 目录时的文件夹名
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: Option<String>,
    pub author: Option<String>,
    pub license: Option<String>,
    pub tags: Vec<String>,
    pub source: SkillSourceInfo,
    /// 本地仓库中的绝对路径
    pub path: String,
    pub file_count: usize,
    pub size_bytes: u64,
    /// 内容指纹，用于判断各 Agent 中的副本是否过期
    pub hash: String,
    pub imported_at: String,
    pub updated_at: String,
}

/// 某个 Skill 在某个 Agent 中的部署情况
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeployInfo {
    pub deployed: bool,
    /// synced | modified | missing
    pub status: String,
    pub path: String,
    pub updated_at: Option<String>,
}

/// 前端使用的 Skill 视图 = 元数据 + 各 Agent 部署状态
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillView {
    #[serde(flatten)]
    pub meta: SkillMeta,
    pub deployments: HashMap<String, DeployInfo>,
}

/// 目标 Agent（Claude Code / Codex / Qoder / OpenCode ...）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentView {
    pub id: String,
    pub name: String,
    pub short: String,
    /// builtin | custom
    pub kind: String,
    /// 解析后的 skills 目录
    pub path: String,
    /// skills 目录是否已存在
    pub ready: bool,
    /// 该 Agent 是否已被检测到安装在系统中
    pub installed: bool,
    pub writable: bool,
    /// 该目录下实际存在的 skill 数量
    pub skill_count: usize,
    /// 由 SkillBox 部署的数量
    pub managed_count: usize,
    pub enabled: bool,
    pub color: String,
    pub homepage: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomAgent {
    pub id: String,
    pub name: String,
    pub path: String,
    pub color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub store_dir: String,
    pub disabled_agents: Vec<String>,
    pub agent_overrides: HashMap<String, String>,
    pub custom_agents: Vec<CustomAgent>,
    pub auto_sync: bool,
    /// light | dark
    pub theme: String,
    /// 部署时若目标已存在是否直接覆盖
    pub overwrite_on_deploy: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            store_dir: String::new(),
            disabled_agents: Vec::new(),
            agent_overrides: HashMap::new(),
            custom_agents: Vec::new(),
            auto_sync: false,
            theme: "light".into(),
            overwrite_on_deploy: true,
        }
    }
}

/// 扫描到的可导入 Skill
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    pub id: String,
    pub name: String,
    pub description: String,
    pub path: String,
    pub rel: String,
    pub has_skill_file: bool,
    pub file_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult {
    pub session: String,
    pub kind: String,
    pub origin: String,
    pub candidates: Vec<Candidate>,
    pub message: Option<String>,
}

/// 单个操作的结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpResult {
    pub ok: bool,
    pub skill_id: String,
    pub agent_id: String,
    pub message: String,
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    pub time: String,
    pub level: String,
    pub message: String,
}

/// 一次返回给前端的完整状态
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StateSnapshot {
    pub skills: Vec<SkillView>,
    pub agents: Vec<AgentView>,
    pub config: Config,
    pub platform: String,
    pub home: String,
}
