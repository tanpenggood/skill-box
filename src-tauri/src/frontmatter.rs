use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub struct SkillDoc {
    pub name: Option<String>,
    pub description: Option<String>,
    pub version: Option<String>,
    pub author: Option<String>,
    pub license: Option<String>,
    pub tags: Vec<String>,
}

/// 极简 YAML frontmatter 解析：只处理 `key: value`、块标量（|、>）和行内数组。
/// 足以覆盖 SKILL.md 的常见写法，同时避免引入完整的 YAML 依赖。
pub fn parse_file(path: &Path) -> SkillDoc {
    let raw = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(_) => return empty_doc(),
    };
    parse_str(&raw)
}

pub fn parse_str(raw: &str) -> SkillDoc {
    let text = raw.trim_start_matches('\u{feff}');
    let lines: Vec<&str> = text.lines().collect();
    if lines.is_empty() || lines[0].trim() != "---" {
        return empty_doc();
    }

    // 找到结束分隔符
    let mut end = None;
    for (i, l) in lines.iter().enumerate().skip(1) {
        let t = l.trim();
        if t == "---" || t == "..." {
            end = Some(i);
            break;
        }
    }
    let end = match end {
        Some(i) => i,
        None => return empty_doc(),
    };

    let body = &lines[1..end];
    let map = parse_block(body);

    let pick = |keys: &[&str]| -> Option<String> {
        for k in keys {
            if let Some(v) = map.get(*k) {
                if !v.trim().is_empty() {
                    return Some(v.trim().to_string());
                }
            }
        }
        None
    };

    let tags = pick(&["tags", "keywords", "tag"])
        .map(|v| {
            let v = v.trim().trim_matches('[').trim_matches(']');
            v.split(',')
                .map(|x| x.trim().trim_matches('"').trim_matches('\'').to_string())
                .filter(|x| !x.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    SkillDoc {
        name: pick(&["name"]),
        description: pick(&["description"]),
        version: pick(&["version", "metadata.version"]),
        author: pick(&["author", "authors"]),
        license: pick(&["license"]),
        tags,
    }
}

fn parse_block(lines: &[&str]) -> HashMap<String, String> {
    let mut map: HashMap<String, String> = HashMap::new();
    let mut i = 0usize;
    while i < lines.len() {
        let line = lines[i];
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            i += 1;
            continue;
        }
        let indent = line.len() - line.trim_start().len();
        if indent > 0 {
            i += 1;
            continue; // 嵌套结构由调用方通过 "a.b" 取值，这里跳过
        }
        if let Some((key, val)) = split_kv(line) {
            let val = val.trim();
            if val == "|" || val == ">" || val == "|-" || val == ">-" || val == "|+" || val == ">+" {
                // 块标量：读取后续缩进行
                let mut buf: Vec<String> = Vec::new();
                let mut j = i + 1;
                while j < lines.len() {
                    let l = lines[j];
                    if l.trim().is_empty() {
                        buf.push(String::new());
                        j += 1;
                        continue;
                    }
                    let li = l.len() - l.trim_start().len();
                    if li <= indent {
                        break;
                    }
                    buf.push(l[indent + 1..].to_string());
                    j += 1;
                }
                let joined = if val.starts_with('|') {
                    buf.join("\n")
                } else {
                    buf.join(" ")
                };
                map.insert(key, joined.trim().to_string());
                i = j;
                continue;
            }
            map.insert(key, unquote(val));
        }
        i += 1;
    }
    map
}

fn split_kv(line: &str) -> Option<(String, String)> {
    let pos = line.find(':')?;
    let key = line[..pos].trim();
    if key.is_empty() || key.contains(':') {
        return None;
    }
    let val = line[pos + 1..].to_string();
    Some((key.to_lowercase(), val))
}

fn unquote(v: &str) -> String {
    let v = v.trim();
    if v.len() >= 2 {
        let first = v.chars().next().unwrap();
        let last = v.chars().last().unwrap();
        if (first == '"' && last == '"') || (first == '\'' && last == '\'') {
            return v[1..v.len() - 1].to_string();
        }
    }
    // 去掉行尾注释
    if let Some(p) = v.find(" #") {
        return v[..p].trim().to_string();
    }
    v.to_string()
}

fn empty_doc() -> SkillDoc {
    SkillDoc {
        name: None,
        description: None,
        version: None,
        author: None,
        license: None,
        tags: Vec::new(),
    }
}
