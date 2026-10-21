use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use flate2::read::GzDecoder;
use walkdir::WalkDir;

use crate::fsutil;
use crate::frontmatter;
use crate::model::{Candidate, ScanResult, SkillMeta, SkillSourceInfo};
use crate::store::Store;

/// 判断目录里是否存在 SKILL.md（忽略大小写）
pub fn skill_file_name(dir: &Path) -> Option<String> {
    let rd = fs::read_dir(dir).ok()?;
    for e in rd.filter_map(|x| x.ok()) {
        let n = e.file_name().to_string_lossy().to_lowercase();
        if n == "skill.md" && e.path().is_file() {
            return Some(e.file_name().to_string_lossy().to_string());
        }
    }
    None
}

fn has_skill_file(dir: &Path) -> bool {
    skill_file_name(dir).is_some()
}

fn skip_dir(name: &str) -> bool {
    matches!(
        name,
        "node_modules" | ".git" | "target" | "dist" | "build" | ".venv" | "venv" | "__pycache__" | ".next" | ".idea"
    ) || name.starts_with('.')
}

/// 在目录树中发现所有 Skill（含 SKILL.md 的文件夹）
pub fn discover(root: &Path) -> Vec<PathBuf> {
    if has_skill_file(root) {
        return vec![root.to_path_buf()];
    }
    let mut found: Vec<PathBuf> = Vec::new();
    let walker = WalkDir::new(root)
        .min_depth(1)
        .max_depth(8)
        .into_iter()
        .filter_entry(|e| {
            if e.file_type().is_dir() {
                !skip_dir(&e.file_name().to_string_lossy())
            } else {
                true
            }
        });
    for entry in walker.filter_map(|x| x.ok()) {
        if entry.file_type().is_dir() && has_skill_file(entry.path()) {
            found.push(entry.path().to_path_buf());
        }
    }
    found.sort();
    // 若父目录已是 skill，则不再收录其子目录
    let mut out: Vec<PathBuf> = Vec::new();
    for p in found {
        if !out.iter().any(|q| p.starts_with(q)) {
            out.push(p);
        }
    }
    out
}

pub fn build_candidates(root: &Path, dirs: &[PathBuf]) -> Vec<Candidate> {
    dirs.iter()
        .map(|d| {
            let rel = d
                .strip_prefix(root)
                .map(|p| p.to_string_lossy().replace('\\', "/"))
                .unwrap_or_else(|_| d.to_string_lossy().to_string());
            let fallback_name = d
                .file_name()
                .map(|x| x.to_string_lossy().to_string())
                .unwrap_or_else(|| "skill".to_string());
            let mut name = fallback_name.clone();
            let mut description = String::new();
            if let Some(fname) = skill_file_name(d) {
                let doc = frontmatter::parse_file(&d.join(fname));
                if let Some(n) = doc.name.filter(|x| !x.trim().is_empty()) {
                    name = n;
                }
                description = doc.description.unwrap_or_default();
            }
            let (file_count, _) = fsutil::dir_stats(d);
            Candidate {
                id: rel.clone(),
                rel,
                name,
                description,
                path: d.to_string_lossy().to_string(),
                has_skill_file: true,
                file_count,
            }
        })
        .collect()
}

// ---------------- GitHub ----------------

#[derive(Debug, Clone)]
pub struct GhRef {
    pub owner: String,
    pub repo: String,
    pub reference: Option<String>,
    pub subpath: Option<String>,
}

pub fn parse_github(input: &str) -> Result<GhRef, String> {
    let s = input.trim().trim_end_matches('/');
    if s.is_empty() {
        return Err("请输入 GitHub 仓库地址".into());
    }
    let mut s = s.to_string();
    if let Some(pos) = s.find("github.com") {
        s = s[pos + "github.com".len()..].trim_start_matches(['/', ':']).to_string();
    }
    let parts: Vec<&str> = s.split('/').filter(|x| !x.is_empty()).collect();
    if parts.len() < 2 {
        return Err(format!("无法解析仓库地址：{input}"));
    }
    let owner = parts[0].to_string();
    let repo = parts[1].trim_end_matches(".git").to_string();
    let mut reference = None;
    let mut subpath = None;
    if parts.len() > 3 && (parts[2] == "tree" || parts[2] == "blob") {
        reference = Some(parts[3].to_string());
        if parts.len() > 4 {
            subpath = Some(parts[4..].join("/"));
        }
    }
    Ok(GhRef {
        owner,
        repo,
        reference,
        subpath,
    })
}

fn candidate_refs(gh: &GhRef) -> Vec<String> {
    let mut v = Vec::new();
    if let Some(r) = &gh.reference {
        v.push(r.clone());
    }
    for d in ["HEAD", "main", "master", "develop"] {
        if !v.contains(&d.to_string()) {
            v.push(d.to_string());
        }
    }
    v
}

fn download_tarball(gh: &GhRef) -> Result<Vec<u8>, String> {
    let mut last_err = String::new();
    for r in candidate_refs(gh) {
        let urls = [
            format!("https://codeload.github.com/{}/{}/tar.gz/{}", gh.owner, gh.repo, r),
            format!(
                "https://github.com/{}/{}/archive/refs/heads/{}.tar.gz",
                gh.owner, gh.repo, r
            ),
        ];
        for url in urls {
            match ureq::get(&url)
                .set("User-Agent", "SkillBox")
                .timeout_connect(15_000)
                .timeout_read(60_000)
                .call()
            {
                Ok(resp) => {
                    let mut buf = Vec::new();
                    match resp.into_reader().read_to_end(&mut buf) {
                        Ok(_) if !buf.is_empty() => return Ok(buf),
                        Ok(_) => last_err = "下载内容为空".into(),
                        Err(e) => last_err = e.to_string(),
                    }
                }
                Err(e) => last_err = format!("{e}"),
            }
        }
    }
    Err(format!(
        "下载 {}/{} 失败，请检查仓库地址、网络或是否需要代理。{}",
        gh.owner,
        gh.repo,
        if last_err.is_empty() {
            String::new()
        } else {
            format!("（{last_err}）")
        }
    ))
}

/// 下载并解包到临时目录，返回解压后的仓库根目录
pub fn fetch_github(gh: &GhRef, tmp_root: &Path, session: &str) -> Result<PathBuf, String> {
    let bytes = download_tarball(gh)?;
    let dest = tmp_root.join(session);
    let _ = fs::remove_dir_all(&dest);
    fs::create_dir_all(&dest).map_err(|e| e.to_string())?;

    let mut ar = tar::Archive::new(GzDecoder::new(&bytes[..]));
    ar.unpack(&dest).map_err(|e| format!("解包失败: {e}"))?;

    // 压缩包里通常有一个顶层目录
    let entries: Vec<PathBuf> = fs::read_dir(&dest)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .collect();
    let root = if entries.len() == 1 && entries[0].is_dir() {
        entries[0].clone()
    } else {
        dest.clone()
    };

    match &gh.subpath {
        Some(sub) => {
            let p = root.join(sub);
            if p.is_dir() {
                Ok(p)
            } else {
                Err(format!("仓库中找不到子目录：{sub}"))
            }
        }
        None => Ok(root),
    }
}

// ---------------- 导入 ----------------

/// 把一组目录复制进本地仓库，生成 SkillMeta
pub fn import_paths(
    store: &Store,
    paths: &[String],
    source: SkillSourceInfo,
) -> Vec<SkillMeta> {
    let mut out = Vec::new();
    for p in paths {
        let src = PathBuf::from(p);
        if !src.is_dir() {
            continue;
        }
        let fallback_name = src
            .file_name()
            .map(|x| x.to_string_lossy().to_string())
            .unwrap_or_else(|| "skill".to_string());
        let mut name = fallback_name.clone();
        let mut description = String::new();
        let mut version = None;
        let mut author = None;
        let mut license = None;
        let mut tags = Vec::new();
        if let Some(f) = skill_file_name(&src) {
            let doc = frontmatter::parse_file(&src.join(f));
            if let Some(n) = doc.name.filter(|x| !x.trim().is_empty()) {
                name = n;
            }
            description = doc.description.unwrap_or_default();
            version = doc.version;
            author = doc.author;
            license = doc.license;
            tags = doc.tags;
        }
        let slug = store.unique_slug(&name);
        let dest = store.skills_dir().join(&slug);
        if fsutil::copy_dir_all(&src, &dest).is_err() {
            continue;
        }
        let (file_count, size_bytes) = fsutil::dir_stats(&dest);
        let now = fsutil::now_string();
        out.push(SkillMeta {
            id: slug,
            name,
            description,
            version,
            author,
            license,
            tags,
            source: source.clone(),
            path: dest.to_string_lossy().to_string(),
            file_count,
            size_bytes,
            hash: fsutil::hash_dir(&dest),
            imported_at: now.clone(),
            updated_at: now,
        });
    }
    out
}

/// 重新计算某个已收录 Skill 的元数据（内容可能已被手动修改）
pub fn refresh_meta(meta: &mut SkillMeta) {
    if let Some(f) = skill_file_name(Path::new(&meta.path)) {
        let doc = frontmatter::parse_file(&Path::new(&meta.path).join(f));
        if let Some(n) = doc.name.filter(|x| !x.trim().is_empty()) {
            meta.name = n;
        }
        meta.description = doc.description.unwrap_or_default();
        meta.version = doc.version;
        meta.author = doc.author;
        meta.license = doc.license;
        meta.tags = doc.tags;
    }
    let (c, s) = fsutil::dir_stats(Path::new(&meta.path));
    meta.file_count = c;
    meta.size_bytes = s;
    meta.hash = fsutil::hash_dir(Path::new(&meta.path));
}

#[allow(dead_code)]
pub fn empty_scan(message: String) -> ScanResult {
    ScanResult {
        session: String::new(),
        kind: "none".into(),
        origin: String::new(),
        candidates: Vec::new(),
        message: Some(message),
    }
}
