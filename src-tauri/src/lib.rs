use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, RwLock};

use tauri::State;

pub mod agents;
pub mod deploy;
pub mod frontmatter;
pub mod fsutil;
pub mod model;
pub mod source;
pub mod store;

use model::{
    Config, LogEntry, OpResult, ScanResult, SkillMeta, SkillSourceInfo, SkillView, StateSnapshot,
};

static SEQ: AtomicU64 = AtomicU64::new(0);

struct SessionInfo {
    root: PathBuf,
    kind: String,
    origin: String,
    owner: Option<String>,
    repo: Option<String>,
    reference: Option<String>,
    subpath: Option<String>,
}

pub struct AppState {
    store_root: RwLock<PathBuf>,
    sessions: Mutex<HashMap<String, SessionInfo>>,
    logs: Mutex<Vec<LogEntry>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            store_root: RwLock::new(agents::default_store_dir()),
            sessions: Mutex::new(HashMap::new()),
            logs: Mutex::new(Vec::new()),
        }
    }

    fn store(&self) -> store::Store {
        let root = self.store_root.read().map(|r| r.clone()).unwrap_or_default();
        store::Store::open(&root.to_string_lossy())
    }

    fn log(&self, level: &str, msg: impl Into<String>) {
        if let Ok(mut l) = self.logs.lock() {
            l.push(LogEntry {
                time: fsutil::now_string(),
                level: level.to_string(),
                message: msg.into(),
            });
            if l.len() > 800 {
                let n = l.len() - 800;
                l.drain(..n);
            }
        }
    }
}

fn all_agent_ids(config: &Config) -> Vec<String> {
    let mut v: Vec<String> = agents::BUILTIN_AGENTS.iter().map(|a| a.id.to_string()).collect();
    v.extend(config.custom_agents.iter().map(|c| c.id.clone()));
    v
}

// ---------------- 命令 ----------------

#[tauri::command]
fn get_state(state: State<'_, AppState>) -> StateSnapshot {
    let st = state.store();
    let config = st.load_config();
    let mut skills = st.load_skills();
    for s in skills.iter_mut() {
        source::refresh_meta(s);
    }
    let _ = st.save_skills(&skills);

    let ids = all_agent_ids(&config);
    let mut managed: HashMap<String, usize> = HashMap::new();
    let mut views: Vec<SkillView> = Vec::new();
    for m in &skills {
        let mut deployments = HashMap::new();
        for a in &ids {
            let info = deploy::status_for(m, a, &config);
            if info.deployed {
                *managed.entry(a.clone()).or_insert(0) += 1;
            }
            deployments.insert(a.clone(), info);
        }
        views.push(SkillView {
            meta: m.clone(),
            deployments,
        });
    }
    views.sort_by(|a, b| a.meta.name.to_lowercase().cmp(&b.meta.name.to_lowercase()));

    StateSnapshot {
        skills: views,
        agents: agents::build_views(&config, &managed),
        config,
        platform: std::env::consts::OS.to_string(),
        home: dirs::home_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default(),
    }
}

#[tauri::command]
fn scan_source(state: State<'_, AppState>, kind: String, input: String) -> Result<ScanResult, String> {
    let st = state.store();
    let session = format!("{}-{}", fsutil::now_secs(), SEQ.fetch_add(1, Ordering::SeqCst));
    let tmp_root = st.tmp_dir();

    let (root, info) = if kind == "github" {
        let gh = source::parse_github(&input)?;
        let root = source::fetch_github(&gh, &tmp_root, &session)?;
        let info = SessionInfo {
            root: root.clone(),
            kind: kind.clone(),
            origin: input.clone(),
            owner: Some(gh.owner.clone()),
            repo: Some(gh.repo.clone()),
            reference: gh.reference.clone(),
            subpath: gh.subpath.clone(),
        };
        (root, info)
    } else {
        let p = PathBuf::from(&input);
        if !p.is_dir() {
            return Err(format!("目录不存在：{input}"));
        }
        let info = SessionInfo {
            root: p.clone(),
            kind: kind.clone(),
            origin: input.clone(),
            owner: None,
            repo: None,
            reference: None,
            subpath: None,
        };
        (p, info)
    };

    let dirs = source::discover(&root);
    let mut candidates = source::build_candidates(&root, &dirs);

    if candidates.is_empty() {
        // 目录里没有 SKILL.md：把它当成"整个目录即一个 skill"来提示，仍允许导入
        candidates.push(model::Candidate {
            id: ".".into(),
            rel: ".".into(),
            name: root
                .file_name()
                .map(|x| x.to_string_lossy().to_string())
                .unwrap_or_else(|| "skill".into()),
            description: "未检测到 SKILL.md，将按原样导入整个目录".into(),
            path: root.to_string_lossy().to_string(),
            has_skill_file: false,
            file_count: fsutil::dir_stats(&root).0,
        });
    }

    if let Ok(mut s) = state.sessions.lock() {
        s.insert(session.clone(), info);
    }

    state.log("info", format!("扫描 {input} 发现 {} 个 Skill", candidates.len()));
    Ok(ScanResult {
        session,
        kind,
        origin: input,
        candidates,
        message: None,
    })
}

#[tauri::command]
fn import_skills(
    state: State<'_, AppState>,
    session: String,
    ids: Vec<String>,
) -> Result<Vec<SkillMeta>, String> {
    let st = state.store();
    let info = {
        let s = state.sessions.lock().map_err(|_| "会话锁失败".to_string())?;
        s.get(&session)
            .map(|x| SessionInfo {
                root: x.root.clone(),
                kind: x.kind.clone(),
                origin: x.origin.clone(),
                owner: x.owner.clone(),
                repo: x.repo.clone(),
                reference: x.reference.clone(),
                subpath: x.subpath.clone(),
            })
            .ok_or_else(|| "会话已过期，请重新扫描".to_string())
    };

    let paths: Vec<String> = ids
        .iter()
        .map(|id| {
            let p = if id == "." {
                info.root.clone()
            } else {
                info.root.join(id)
            };
            p.to_string_lossy().to_string()
        })
        .collect();

    let src_kind = if info.kind == "github" { "github" } else { "local" };
    let new_subpath = info.subpath.clone();
    let origin = info.origin.clone();

    let mut metas = source::import_paths(&st, &paths, SkillSourceInfo {
        kind: src_kind.to_string(),
        origin: Some(origin),
        owner: info.owner.clone(),
        repo: info.repo.clone(),
        reference: info.reference.clone(),
        subpath: new_subpath,
    });

    // GitHub 导入时，把 skill 在仓库中的相对路径记下来，便于后续"一键更新"
    if src_kind == "github" {
        for (i, m) in metas.iter_mut().enumerate() {
            if let Some(rel) = ids.get(i) {
                let combined = match &info.subpath {
                    Some(sub) => format!("{sub}/{}", rel.trim_start_matches('.')),
                    None => rel.clone(),
                };
                m.source.subpath = Some(combined.trim_matches('/').to_string());
            }
        }
    }

    let mut list = st.load_skills();
    list.extend(metas.clone());
    let _ = st.save_skills(&list);
    state.log("info", format!("导入 {} 个 Skill", metas.len()));

    // 清理临时目录
    if info.kind == "github" {
        let tmp = st.tmp_dir().join(&session);
        let _ = fsutil::remove_dir_all_force(&tmp);
    }
    if let Ok(mut s) = state.sessions.lock() {
        s.remove(&session);
    }
    Ok(metas)
}

#[tauri::command]
fn cancel_scan(state: State<'_, AppState>, session: String) -> Result<(), String> {
    let st = state.store();
    let tmp = st.tmp_dir().join(&session);
    let _ = fsutil::remove_dir_all_force(&tmp);
    if let Ok(mut s) = state.sessions.lock() {
        s.remove(&session);
    }
    Ok(())
}

#[tauri::command]
fn delete_skill(state: State<'_, AppState>, id: String, undeploy_all: bool) -> Result<Vec<OpResult>, String> {
    let st = state.store();
    let config = st.load_config();
    let list = st.load_skills();
    let meta = list.iter().find(|s| s.id == id).cloned();
    let mut ops = Vec::new();
    if let Some(m) = meta {
        if undeploy_all {
            for a in all_agent_ids(&config) {
                ops.push(deploy::undeploy(&m, &a, &config));
            }
        }
        if let Err(e) = fsutil::remove_dir_all_force(&PathBuf::from(&m.path)) {
            ops.push(OpResult {
                ok: false,
                skill_id: id.clone(),
                agent_id: "*".into(),
                message: e,
                path: None,
            });
        }
    }
    let list: Vec<SkillMeta> = list.into_iter().filter(|s| s.id != id).collect();
    let _ = st.save_skills(&list);
    state.log("info", format!("移除 Skill：{id}"));
    Ok(ops)
}

#[tauri::command]
fn deploy_selection(
    state: State<'_, AppState>,
    skill_ids: Vec<String>,
    agent_ids: Vec<String>,
    overwrite: Option<bool>,
) -> Result<Vec<OpResult>, String> {
    let st = state.store();
    let config = st.load_config();
    let overwrite = overwrite.unwrap_or(config.overwrite_on_deploy);
    let list = st.load_skills();
    let mut ops = Vec::new();
    for sid in &skill_ids {
        let meta = match list.iter().find(|s| &s.id == sid) {
            Some(m) => m.clone(),
            None => {
                ops.push(OpResult {
                    ok: false,
                    skill_id: sid.clone(),
                    agent_id: "*".into(),
                    message: "Skill 不存在".into(),
                    path: None,
                });
                continue;
            }
        };
        for aid in &agent_ids {
            ops.push(deploy::deploy(&meta, aid, &config, overwrite));
        }
    }
    let ok = ops.iter().filter(|o| o.ok).count();
    state.log("info", format!("部署操作完成：成功 {ok} / 共 {}", ops.len()));
    Ok(ops)
}

#[tauri::command]
fn undeploy_selection(
    state: State<'_, AppState>,
    skill_ids: Vec<String>,
    agent_ids: Vec<String>,
) -> Result<Vec<OpResult>, String> {
    let st = state.store();
    let config = st.load_config();
    let list = st.load_skills();
    let mut ops = Vec::new();
    for sid in &skill_ids {
        let meta = match list.iter().find(|s| &s.id == sid) {
            Some(m) => m.clone(),
            None => continue,
        };
        for aid in &agent_ids {
            ops.push(deploy::undeploy(&meta, aid, &config));
        }
    }
    let ok = ops.iter().filter(|o| o.ok).count();
    state.log("info", format!("卸载操作完成：成功 {ok} / 共 {}", ops.len()));
    Ok(ops)
}

/// 把所有已部署的副本重新同步为本地仓库中的最新内容
#[tauri::command]
fn sync_deployed(state: State<'_, AppState>, agent_ids: Vec<String>) -> Result<Vec<OpResult>, String> {
    let st = state.store();
    let config = st.load_config();
    let list = st.load_skills();
    let targets = if agent_ids.is_empty() {
        all_agent_ids(&config)
    } else {
        agent_ids
    };
    let mut ops = Vec::new();
    for m in &list {
        for a in &targets {
            let info = deploy::status_for(m, a, &config);
            if info.deployed {
                ops.push(deploy::deploy(m, a, &config, true));
            }
        }
    }
    let ok = ops.iter().filter(|o| o.ok).count();
    state.log("info", format!("同步完成：{ok} 项"));
    Ok(ops)
}

/// 对来自 GitHub 的 Skill 重新拉取并覆盖本地正本
#[tauri::command]
fn update_from_source(state: State<'_, AppState>, ids: Vec<String>) -> Result<Vec<OpResult>, String> {
    let st = state.store();
    let mut list = st.load_skills();
    let mut ops = Vec::new();
    for id in ids {
        let idx = match list.iter().position(|s| s.id == id) {
            Some(i) => i,
            None => continue,
        };
        let meta = list[idx].clone();
        if meta.source.kind != "github" {
            ops.push(OpResult {
                ok: false,
                skill_id: id,
                agent_id: "*".into(),
                message: "该 Skill 不是来自 GitHub".into(),
                path: None,
            });
            continue;
        }
        let owner = meta.source.owner.clone().unwrap_or_default();
        let repo = meta.source.repo.clone().unwrap_or_default();
        let gh = source::GhRef {
            owner: owner.clone(),
            repo: repo.clone(),
            reference: meta.source.reference.clone(),
            subpath: None,
        };
        let session = format!("upd-{}-{}", fsutil::now_secs(), SEQ.fetch_add(1, Ordering::SeqCst));
        match source::fetch_github(&gh, &st.tmp_dir(), &session) {
            Ok(root) => {
                let rel = meta.source.subpath.clone().unwrap_or_default();
                let src = if rel.is_empty() { root.clone() } else { root.join(&rel) };
                let dest = PathBuf::from(&meta.path);
                let r = fsutil::remove_dir_all_force(&dest)
                    .and_then(|_| fsutil::copy_dir_all(&src, &dest));
                match r {
                    Ok(_) => {
                        let mut m = meta.clone();
                        source::refresh_meta(&mut m);
                        m.updated_at = fsutil::now_string();
                        list[idx] = m.clone();
                        ops.push(OpResult {
                            ok: true,
                            skill_id: id.clone(),
                            agent_id: "*".into(),
                            message: format!("已更新到 {owner}/{repo} 最新内容"),
                            path: Some(m.path.clone()),
                        });
                    }
                    Err(e) => ops.push(OpResult {
                        ok: false,
                        skill_id: id.clone(),
                        agent_id: "*".into(),
                        message: e,
                        path: None,
                    }),
                }
                let _ = fsutil::remove_dir_all_force(&st.tmp_dir().join(&session));
            }
            Err(e) => ops.push(OpResult {
                ok: false,
                skill_id: id.clone(),
                agent_id: "*".into(),
                message: e,
                path: None,
            }),
        }
    }
    let _ = st.save_skills(&list);
    Ok(ops)
}

#[tauri::command]
fn save_config(state: State<'_, AppState>, config: Config) -> Result<Config, String> {
    let st = state.store();
    if let Ok(mut r) = state.store_root.write() {
        if !config.store_dir.trim().is_empty() {
            *r = PathBuf::from(&config.store_dir);
        }
    }
    let st2 = state.store();
    st2.save_config(&config)?;
    state.log("info", "配置已保存");
    Ok(config)
}

#[tauri::command]
fn add_custom_agent(state: State<'_, AppState>, name: String, path: String) -> Result<Config, String> {
    let st = state.store();
    let mut config = st.load_config();
    let ca = agents::make_custom(name.clone(), path);
    config.custom_agents.retain(|c| c.id != ca.id);
    config.custom_agents.push(ca);
    st.save_config(&config)?;
    state.log("info", format!("新增自定义 Agent：{name}"));
    Ok(config)
}

#[tauri::command]
fn remove_custom_agent(state: State<'_, AppState>, id: String) -> Result<Config, String> {
    let st = state.store();
    let mut config = st.load_config();
    config.custom_agents.retain(|c| c.id != id);
    st.save_config(&config)?;
    Ok(config)
}

#[tauri::command]
fn set_agent_path(state: State<'_, AppState>, id: String, path: String) -> Result<Config, String> {
    let st = state.store();
    let mut config = st.load_config();
    if let Some(ca) = config.custom_agents.iter_mut().find(|c| c.id == id) {
        ca.path = path.clone();
    } else {
        config.agent_overrides.insert(id.clone(), path.clone());
    }
    st.save_config(&config)?;
    state.log("info", format!("更新 {id} 的 skills 目录 -> {path}"));
    Ok(config)
}

#[tauri::command]
fn toggle_agent(state: State<'_, AppState>, id: String, enabled: bool) -> Result<Config, String> {
    let st = state.store();
    let mut config = st.load_config();
    config.disabled_agents.retain(|x| x != &id);
    if !enabled {
        config.disabled_agents.push(id);
    }
    st.save_config(&config)?;
    Ok(config)
}

#[tauri::command]
fn reveal_path(state: State<'_, AppState>, path: String) -> Result<(), String> {
    state.log("info", format!("打开目录：{path}"));
    fsutil::reveal(&path)
}

#[tauri::command]
fn get_logs(state: State<'_, AppState>) -> Vec<LogEntry> {
    state
        .logs
        .lock()
        .map(|l| l.iter().rev().take(200).cloned().collect())
        .unwrap_or_default()
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            get_state,
            scan_source,
            import_skills,
            cancel_scan,
            delete_skill,
            deploy_selection,
            undeploy_selection,
            sync_deployed,
            update_from_source,
            save_config,
            add_custom_agent,
            remove_custom_agent,
            set_agent_path,
            toggle_agent,
            reveal_path,
            get_logs
        ])
        .run(tauri::generate_context!())
        .expect("SkillBox 启动失败");
}
