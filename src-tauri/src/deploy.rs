use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use crate::agents;
use crate::fsutil;
use crate::model::{Config, DeployInfo, OpResult, SkillMeta};

/// 计算某个 skill 在某个 agent 中的部署状态
pub fn status_for(meta: &SkillMeta, agent_id: &str, config: &Config) -> DeployInfo {
    let base = match agents::target_dir(agent_id, config) {
        Ok(p) => p,
        Err(_) => {
            return DeployInfo {
                deployed: false,
                status: "missing".into(),
                path: String::new(),
                updated_at: None,
            }
        }
    };
    let target = base.join(&meta.id);
    if !target.exists() {
        return DeployInfo {
            deployed: false,
            status: "missing".into(),
            path: target.to_string_lossy().to_string(),
            updated_at: None,
        };
    }
    let deployed_hash = fsutil::hash_dir(&target);
    let status = if deployed_hash == meta.hash {
        "synced"
    } else {
        "modified"
    };
    DeployInfo {
        deployed: true,
        status: status.into(),
        path: target.to_string_lossy().to_string(),
        updated_at: fs::metadata(&target)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| fsutil::format_ts(d.as_secs())),
    }
}

fn op(ok: bool, skill_id: &str, agent_id: &str, message: impl Into<String>, path: Option<String>) -> OpResult {
    OpResult {
        ok,
        skill_id: skill_id.to_string(),
        agent_id: agent_id.to_string(),
        message: message.into(),
        path,
    }
}

/// 部署：把本地仓库中的正本复制到目标 Agent 的 skills 目录
pub fn deploy(meta: &SkillMeta, agent_id: &str, config: &Config, overwrite: bool) -> OpResult {
    let base = match agents::target_dir(agent_id, config) {
        Ok(p) => p,
        Err(e) => return op(false, &meta.id, agent_id, e, None),
    };
    let target = base.join(&meta.id);
    if target.exists() {
        if !overwrite {
            return op(
                false,
                &meta.id,
                agent_id,
                "目标已存在，未覆盖",
                Some(target.to_string_lossy().to_string()),
            );
        }
        if let Err(e) = fsutil::remove_dir_all_force(&target) {
            return op(false, &meta.id, agent_id, e, None);
        }
    }
    if let Err(e) = fs::create_dir_all(&base) {
        return op(false, &meta.id, agent_id, format!("创建目录失败：{e}"), None);
    }
    match fsutil::copy_dir_all(Path::new(&meta.path), &target) {
        Ok(_) => op(
            true,
            &meta.id,
            agent_id,
            "已部署",
            Some(target.to_string_lossy().to_string()),
        ),
        Err(e) => op(false, &meta.id, agent_id, e, None),
    }
}

/// 卸载：从目标 Agent 的 skills 目录移除
pub fn undeploy(meta: &SkillMeta, agent_id: &str, config: &Config) -> OpResult {
    let base = match agents::target_dir(agent_id, config) {
        Ok(p) => p,
        Err(e) => return op(false, &meta.id, agent_id, e, None),
    };
    let target = base.join(&meta.id);
    if !target.exists() {
        return op(false, &meta.id, agent_id, "未部署，无需卸载", None);
    }
    // 安全检查：目标必须是「agent 目录 / skill id」，避免误删
    if !is_child_of(&target, &base) {
        return op(false, &meta.id, agent_id, "目标路径异常，已取消", None);
    }
    match fsutil::remove_dir_all_force(&target) {
        Ok(_) => op(true, &meta.id, agent_id, "已卸载", None),
        Err(e) => op(false, &meta.id, agent_id, e, None),
    }
}

fn is_child_of(child: &Path, parent: &Path) -> bool {
    match (child.canonicalize(), parent.canonicalize()) {
        (Ok(c), Ok(p)) => c.starts_with(p) && c != p,
        _ => false,
    }
}

/// 列出 Agent 目录下所有由 SkillBox 管理的 skill id
pub fn managed_ids(agent_id: &str, config: &Config, known: &[String]) -> Vec<String> {
    let base = match agents::target_dir(agent_id, config) {
        Ok(p) => p,
        Err(_) => return Vec::new(),
    };
    known
        .iter()
        .filter(|id| base.join(id).exists())
        .cloned()
        .collect()
}

pub fn skill_dirs_in(dir: &PathBuf) -> Vec<String> {
    if !dir.is_dir() {
        return Vec::new();
    }
    fs::read_dir(dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .map(|e| e.file_name().to_string_lossy().to_string())
                .collect()
        })
        .unwrap_or_default()
}
