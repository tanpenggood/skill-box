use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use sha2::{Digest, Sha256};
use walkdir::WalkDir;

const SKIP_DIRS: [&str; 10] = [
    "node_modules",
    ".git",
    "target",
    "dist",
    "build",
    ".venv",
    "venv",
    "__pycache__",
    ".next",
    ".idea",
];

pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// 秒级时间戳 -> "YYYY-MM-DD HH:MM:SS"（不依赖 chrono）
pub fn format_ts(secs: u64) -> String {
    let days = (secs / 86400) as i64;
    let sod = (secs % 86400) as i64;
    let (y, m, d) = civil_from_days(days);
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        y,
        m,
        d,
        sod / 3600,
        (sod % 3600) / 60,
        sod % 60
    )
}

pub fn now_string() -> String {
    format_ts(now_secs())
}

fn civil_from_days(z: i64) -> (i64, u64, u64) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u64;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u64;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// 递归复制目录
pub fn copy_dir_all(src: &Path, dst: &Path) -> Result<(), String> {
    if !src.is_dir() {
        return Err(format!("源目录不存在: {}", src.display()));
    }
    fs::create_dir_all(dst).map_err(|e| format!("创建目录失败 {}: {e}", dst.display()))?;
    for entry in WalkDir::new(src).min_depth(1) {
        let entry = entry.map_err(|e| e.to_string())?;
        let rel = entry
            .path()
            .strip_prefix(src)
            .map_err(|_| "路径前缀剥离失败".to_string())?;
        let target = dst.join(rel);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&target)
                .map_err(|e| format!("创建目录失败 {}: {e}", target.display()))?;
        } else if entry.file_type().is_file() {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).ok();
            }
            fs::copy(entry.path(), &target)
                .map_err(|e| format!("复制文件失败 {}: {e}", entry.path().display()))?;
        }
    }
    Ok(())
}

/// 尽量彻底地删除目录（Windows 上处理只读属性）
pub fn remove_dir_all_force(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }
    make_writable(path);
    match fs::remove_dir_all(path) {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("删除失败 {}: {e}", path.display())),
    }
}

fn make_writable(path: &Path) {
    if path.is_dir() {
        for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
            if let Ok(meta) = entry.metadata() {
                if meta.permissions().readonly() {
                    let mut perm = meta.permissions();
                    #[allow(clippy::permissions_set_readonly_false)]
                    perm.set_readonly(false);
                    let _ = fs::set_permissions(entry.path(), perm);
                }
            }
        }
    }
}

/// 目录指纹：按相对路径排序后，对「路径 + 内容」做 sha256
pub fn hash_dir(dir: &Path) -> String {
    let mut hasher = Sha256::new();
    let mut items: Vec<PathBuf> = Vec::new();
    for entry in WalkDir::new(dir)
        .min_depth(1)
        .into_iter()
        .filter_entry(|e| !is_skipped_dir(e))
    {
        if let Ok(e) = entry {
            if e.file_type().is_file() {
                items.push(e.path().to_path_buf());
            }
        }
    }
    items.sort();
    for p in items {
        let rel = p.strip_prefix(dir).unwrap_or(&p).to_string_lossy().replace('\\', "/");
        hasher.update(rel.as_bytes());
        hasher.update(&[0u8]);
        if let Ok(mut f) = fs::File::open(&p) {
            let mut buf = Vec::new();
            if f.read_to_end(&mut buf).is_ok() {
                hasher.update(&buf);
            }
        }
        hasher.update(&[1u8]);
    }
    format!("{:x}", hasher.finalize())
}

/// 统计目录下文件数量与总字节
pub fn dir_stats(dir: &Path) -> (usize, u64) {
    let mut count = 0usize;
    let mut size = 0u64;
    for entry in WalkDir::new(dir)
        .min_depth(1)
        .into_iter()
        .filter_entry(|e| !is_skipped_dir(e))
    {
        if let Ok(e) = entry {
            if e.file_type().is_file() {
                count += 1;
                if let Ok(m) = e.metadata() {
                    size += m.len();
                }
            }
        }
    }
    (count, size)
}

fn is_skipped_dir(e: &walkdir::DirEntry) -> bool {
    if !e.file_type().is_dir() {
        return false;
    }
    let name = e.file_name().to_string_lossy().to_string();
    SKIP_DIRS.contains(&name.as_str())
}

pub fn is_hidden_or_skipped(name: &str) -> bool {
    SKIP_DIRS.contains(&name) || name.starts_with('.')
}

/// 把任意字符串转成安全的 slug
pub fn slugify(input: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;
    for ch in input.trim().to_lowercase().chars() {
        let ok = ch.is_ascii_alphanumeric() || ch == '-' || ch == '_';
        if ok {
            out.push(ch);
            last_dash = false;
        } else if !last_dash && !out.is_empty() {
            out.push('-');
            last_dash = true;
        }
    }
    let out = out.trim_matches('-').to_string();
    if out.is_empty() {
        "skill".to_string()
    } else {
        out
    }
}

/// 在文件管理器中打开路径
pub fn reveal(path: &str) -> Result<(), String> {
    let p = PathBuf::from(path);
    let target: PathBuf = if p.is_dir() {
        p.clone()
    } else {
        p.parent().map(|x| x.to_path_buf()).unwrap_or(p.clone())
    };
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&target)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&target)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    {
        std::process::Command::new("xdg-open")
            .arg(&target)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}
