use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::model::{AgentView, Config, CustomAgent};

pub struct AgentDef {
    pub id: &'static str,
    pub name: &'static str,
    pub short: &'static str,
    pub color: &'static str,
    pub homepage: Option<&'static str>,
    /// 候选 skills 目录，按优先级排列。
    /// `~` = 用户目录，`%CONFIG%` = 系统配置目录（Windows: %APPDATA%），`%DATA%` = 数据目录
    pub candidates: &'static [&'static str],
}

pub const BUILTIN_AGENTS: &[AgentDef] = &[
    AgentDef {
        id: "claude-code",
        name: "Claude Code",
        short: "CC",
        color: "#D97757",
        homepage: Some("https://docs.claude.com/en/docs/claude-code/skills"),
        candidates: &["~/.claude/skills"],
    },
    AgentDef {
        id: "codex",
        name: "Codex CLI",
        short: "CX",
        color: "#10A37F",
        homepage: Some("https://github.com/openai/codex"),
        candidates: &["~/.codex/skills"],
    },
    AgentDef {
        id: "qoder",
        name: "Qoder",
        short: "QD",
        color: "#4F7CFF",
        homepage: Some("https://qoder.com"),
        candidates: &["~/.qoder/skills"],
    },
    AgentDef {
        id: "opencode",
        name: "OpenCode",
        short: "OC",
        color: "#7C5CFF",
        homepage: Some("https://opencode.ai"),
        candidates: &[
            "~/.config/opencode/skill",
            "~/.config/opencode/skills",
            "%CONFIG%/opencode/skill",
            "%CONFIG%/opencode/skills",
        ],
    },
    AgentDef {
        id: "cursor",
        name: "Cursor",
        short: "CU",
        color: "#5B5BD6",
        homepage: Some("https://cursor.com"),
        candidates: &["~/.cursor/skills"],
    },
    AgentDef {
        id: "gemini-cli",
        name: "Gemini CLI",
        short: "GM",
        color: "#4285F4",
        homepage: Some("https://github.com/google-gemini/gemini-cli"),
        candidates: &["~/.gemini/skills"],
    },
    AgentDef {
        id: "windsurf",
        name: "Windsurf",
        short: "WS",
        color: "#0EA5E9",
        homepage: Some("https://windsurf.com"),
        candidates: &["~/.codeium/windsurf/skills"],
    },
    AgentDef {
        id: "trae",
        name: "Trae",
        short: "TR",
        color: "#7C3AED",
        homepage: Some("https://www.trae.ai"),
        candidates: &["~/.trae/skills"],
    },
    AgentDef {
        id: "github-copilot",
        name: "GitHub Copilot CLI",
        short: "CP",
        color: "#3B3B45",
        homepage: Some("https://github.com/github/copilot-cli"),
        candidates: &["~/.copilot/skills"],
    },
    AgentDef {
        id: "workbuddy",
        name: "WorkBuddy / CodeBuddy",
        short: "WB",
        color: "#00B96B",
        homepage: Some("https://www.codebuddy.cn"),
        candidates: &["~/.workbuddy/skills", "~/.codebuddy/skills"],
    },
];

pub fn default_store_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".skillbox")
}

/// 展开路径模板
pub fn expand(tpl: &str) -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    let s = tpl.trim();
    if let Some(rest) = s.strip_prefix("~") {
        let rest = rest.trim_start_matches(['/', '\\']);
        return Some(if rest.is_empty() {
            home.clone()
        } else {
            home.join(rest.replace('/', std::path::MAIN_SEPARATOR.to_string().as_str()))
        });
    }
    if let Some(rest) = s.strip_prefix("%CONFIG%") {
        if let Some(c) = dirs::config_dir() {
            let rest = rest.trim_start_matches(['/', '\\']);
            return Some(if rest.is_empty() {
                c
            } else {
                c.join(rest.replace('/', std::path::MAIN_SEPARATOR.to_string().as_str()))
            });
        }
    }
    if let Some(rest) = s.strip_prefix("%DATA%") {
        if let Some(c) = dirs::data_dir() {
            let rest = rest.trim_start_matches(['/', '\\']);
            return Some(if rest.is_empty() {
                c
            } else {
                c.join(rest.replace('/', std::path::MAIN_SEPARATOR.to_string().as_str()))
            });
        }
    }
    Some(PathBuf::from(s))
}

struct Resolved {
    path: PathBuf,
    ready: bool,
    installed: bool,
}

fn resolve_for(def: &AgentDef, overrides: &HashMap<String, String>) -> Resolved {
    if let Some(p) = overrides.get(def.id) {
        let p = PathBuf::from(p);
        return Resolved {
            installed: p.parent().map(|x| x.exists()).unwrap_or(false) || p.exists(),
            ready: p.exists(),
            path: p,
        };
    }
    let expanded: Vec<PathBuf> = def.candidates.iter().filter_map(|c| expand(c)).collect();
    for p in &expanded {
        if p.exists() {
            return Resolved {
                ready: true,
                installed: true,
                path: p.clone(),
            };
        }
    }
    // 没有任何已存在的目录：用第一个候选作为默认目标，
    // 同时判断其父目录（Agent 主目录）是否存在，用来提示"已安装但未初始化 skills 目录"
    let first = expanded.first().cloned().unwrap_or_else(|| PathBuf::from("."));
    let parent_installed = expanded.iter().any(|p| {
        p.parent().map(|x| x.exists()).unwrap_or(false)
    });
    Resolved {
        ready: first.exists(),
        installed: parent_installed || first.exists(),
        path: first,
    }
}

/// 扫描某个 skills 目录下的 skill 数量（含子目录）
pub fn count_skills(dir: &Path) -> usize {
    if !dir.is_dir() {
        return 0;
    }
    std::fs::read_dir(dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .count()
        })
        .unwrap_or(0)
}

/// 生成给前端的 Agent 列表
pub fn build_views(config: &Config, managed: &HashMap<String, usize>) -> Vec<AgentView> {
    let mut out: Vec<AgentView> = Vec::new();
    for def in BUILTIN_AGENTS {
        let r = resolve_for(def, &config.agent_overrides);
        out.push(AgentView {
            id: def.id.to_string(),
            name: def.name.to_string(),
            short: def.short.to_string(),
            kind: "builtin".into(),
            path: r.path.to_string_lossy().to_string(),
            ready: r.ready,
            installed: r.installed,
            writable: writable(&r.path),
            skill_count: count_skills(&r.path),
            managed_count: *managed.get(def.id).unwrap_or(&0),
            enabled: !config.disabled_agents.contains(&def.id.to_string()),
            color: def.color.to_string(),
            homepage: def.homepage.map(|s| s.to_string()),
        });
    }
    for ca in &config.custom_agents {
        let p = PathBuf::from(&ca.path);
        out.push(AgentView {
            id: ca.id.clone(),
            name: ca.name.clone(),
            short: ca
                .name
                .chars()
                .filter(|c| c.is_alphanumeric())
                .take(2)
                .collect::<String>()
                .to_uppercase(),
            kind: "custom".into(),
            path: ca.path.clone(),
            ready: p.exists(),
            installed: p.parent().map(|x| x.exists()).unwrap_or(false) || p.exists(),
            writable: writable(&p),
            skill_count: count_skills(&p),
            managed_count: *managed.get(&ca.id).unwrap_or(&0),
            enabled: !config.disabled_agents.contains(&ca.id),
            color: ca.color.clone(),
            homepage: None,
        });
    }
    out
}

pub fn find_def(id: &str) -> Option<&'static AgentDef> {
    BUILTIN_AGENTS.iter().find(|a| a.id == id)
}

/// 得到某个 Agent 最终的目标 skills 目录
pub fn target_dir(id: &str, config: &Config) -> Result<PathBuf, String> {
    if let Some(ca) = config.custom_agents.iter().find(|c| c.id == id) {
        return Ok(PathBuf::from(&ca.path));
    }
    let def = find_def(id).ok_or_else(|| format!("未找到 Agent: {id}"))?;
    Ok(resolve_for(def, &config.agent_overrides).path)
}

fn writable(p: &Path) -> bool {
    if p.exists() {
        // 已存在：尝试用临时文件探测写权限
        let probe = p.join(".skillbox_write_probe");
        match std::fs::write(&probe, b"") {
            Ok(_) => {
                let _ = std::fs::remove_file(&probe);
                true
            }
            Err(_) => false,
        }
    } else {
        p.parent().map(|x| x.exists()).unwrap_or(false)
    }
}

pub fn custom_id(name: &str) -> String {
    format!("custom-{}", crate::fsutil::slugify(name))
}

pub fn make_custom(name: String, path: String) -> CustomAgent {
    CustomAgent {
        id: custom_id(&name),
        name,
        path,
        color: "#64748B".to_string(),
    }
}
