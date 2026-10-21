use std::fs;
use std::path::{Path, PathBuf};

use crate::fsutil;
use crate::model::{Config, SkillMeta};

/// SkillBox 的本地仓库：所有被管理的 Skill 都在这里保留一份正本
pub struct Store {
    pub root: PathBuf,
}

impl Store {
    pub fn open(configured: &str) -> Self {
        let root = if configured.trim().is_empty() {
            crate::agents::default_store_dir()
        } else {
            PathBuf::from(configured)
        };
        let _ = fs::create_dir_all(&root);
        let _ = fs::create_dir_all(root.join("skills"));
        let _ = fs::create_dir_all(root.join("tmp"));
        Self { root }
    }

    pub fn skills_dir(&self) -> PathBuf {
        self.root.join("skills")
    }

    pub fn tmp_dir(&self) -> PathBuf {
        self.root.join("tmp")
    }

    pub fn config_path(&self) -> PathBuf {
        self.root.join("config.json")
    }

    pub fn index_path(&self) -> PathBuf {
        self.root.join("skills.json")
    }

    pub fn load_config(&self) -> Config {
        let mut cfg: Config = fs::read_to_string(self.config_path())
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        if cfg.store_dir.trim().is_empty() {
            cfg.store_dir = self.root.to_string_lossy().to_string();
        }
        cfg
    }

    pub fn save_config(&self, cfg: &Config) -> Result<(), String> {
        let s = serde_json::to_string_pretty(cfg).map_err(|e| e.to_string())?;
        fs::write(self.config_path(), s).map_err(|e| e.to_string())
    }

    /// 读取索引，并自动剔除磁盘上已不存在的条目
    pub fn load_skills(&self) -> Vec<SkillMeta> {
        let list: Vec<SkillMeta> = fs::read_to_string(self.index_path())
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        list.into_iter()
            .filter(|s| Path::new(&s.path).join("SKILL.md").exists() || Path::new(&s.path).is_dir())
            .collect()
    }

    pub fn save_skills(&self, list: &[SkillMeta]) -> Result<(), String> {
        let s = serde_json::to_string_pretty(list).map_err(|e| e.to_string())?;
        fs::write(self.index_path(), s).map_err(|e| e.to_string())
    }

    /// 为导入生成一个唯一的 slug
    pub fn unique_slug(&self, base: &str) -> String {
        let base = fsutil::slugify(base);
        let existing: Vec<String> = self
            .load_skills()
            .into_iter()
            .map(|s| s.id)
            .collect();
        let mut candidate = base.clone();
        let mut i = 2;
        while existing.contains(&candidate) || self.skills_dir().join(&candidate).exists() {
            candidate = format!("{base}-{i}");
            i += 1;
        }
        candidate
    }
}
