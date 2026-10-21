# SkillBox · 跨 Agent 的 AI Skills 管理器

一次编码，Windows / macOS / Linux 通用的桌面应用。把散落在各处的 Skill 收进一个本地仓库，
然后**一键部署到任意多个 Agent**（Claude Code、Codex、Qoder、OpenCode、Cursor…），
也可以按 Agent 单独卸载。

技术栈：**Tauri v2 + Rust + React 18 + TypeScript + Vite**（一份代码，三平台原生安装包）。

---

## 核心能力

| 能力 | 说明 |
| --- | --- |
| 本地仓库 | 所有导入的 Skill 在 `~/.skillbox/skills/<slug>/` 保留一份正本，部署时复制分发，互不干扰 |
| 本地目录导入 | 选一个 skill 目录，或选包含多个 skill 的父目录，自动向下查找 `SKILL.md` |
| GitHub 导入 | 支持 `owner/repo`、完整 URL、`/tree/分支/子目录`，自动下载解包并列出全部可导入项供勾选 |
| 一键部署 | 勾选多个 Skill → 选多个 Agent → 一次写入 |
| 精准卸载 | 按「Skill × Agent」粒度卸载，不影响其他 Agent |
| 状态比对 | 用内容哈希比对正本与各 Agent 副本，标出「已同步 / 已改动 / 未部署」 |
| 一键同步 | 把所有已部署副本刷新为仓库最新内容 |
| 拉取更新 | GitHub 来源的 Skill 可重新拉取上游并覆盖正本 |
| 部署矩阵 | 表格化批量勾选，先记录变更再一次性应用 |
| 自定义 Agent | 内置列表里没有的工具，直接指定它的 skills 目录即可 |
| 跨平台 | `npm run tauri:build` 或 GitHub Actions 自动产出三平台安装包 |

## 内置 Agent 及默认 skills 目录

| Agent | 默认目录 |
| --- | --- |
| Claude Code | `~/.claude/skills` |
| Codex CLI | `~/.codex/skills` |
| Qoder | `~/.qoder/skills` |
| OpenCode | `~/.config/opencode/skill`（Windows 下会回退到 `%APPDATA%/opencode/skill`） |
| Cursor | `~/.cursor/skills` |
| Gemini CLI | `~/.gemini/skills` |
| Windsurf | `~/.codeium/windsurf/skills` |
| Trae | `~/.trae/skills` |
| GitHub Copilot CLI | `~/.copilot/skills` |
| WorkBuddy / CodeBuddy | `~/.workbuddy/skills` |

目录不对？在「Agent 管理」里点「修改」自定义路径，配置保存在 `~/.skillbox/config.json`。

## 快速开始

```bash
npm install
npm run tauri:dev      # 开发模式
npm run tauri:build    # 打包当前平台安装包
```

### Windows 前置条件

Tauri 在 Windows 上必须使用 MSVC 链接器，需要 **Visual Studio 的「使用 C++ 的桌面开发」工作负载**。
一键脚本会帮你检查并引导安装：

```powershell
Set-ExecutionPolicy -Scope Process Bypass
.\scripts\setup-windows.ps1
```

已装 VS Community 的话，补装工作负载即可：
Visual Studio Installer → 修改 → 勾选「使用 C++ 的桌面开发」→ 安装。

### macOS / Linux

macOS 需要 Xcode Command Line Tools（`xcode-select --install`）。
Linux 需要 WebKit2GTK 等系统依赖：

```bash
sudo apt-get install -y libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf
```

## 跨平台打包

推送 tag 即触发 GitHub Actions（`.github/workflows/release.yml`），在云端并行构建：

- Windows：`.msi` + `.exe`（NSIS）
- macOS：Apple Silicon 与 Intel 两份 `.dmg`
- Linux：`.AppImage` + `.deb`

```bash
git tag v0.1.0 && git push origin v0.1.0
```

## Skill 目录约定

一个 Skill 就是一个含 `SKILL.md` 的文件夹：

```
my-skill/
├── SKILL.md          # 必需，含 YAML frontmatter
├── scripts/
│   └── run.py
└── references/
    └── api.md
```

`SKILL.md` 示例：

```markdown
---
name: my-skill
description: 一句话说明这个 skill 做什么、什么时候该用它
version: 1.0.0
author: tanpenggood
tags: [git, automation]
---

# 正文写具体用法…
```

`name` 决定 slug（也是部署到各 Agent 时的文件夹名），缺失时用目录名兜底。

## 数据存储

```
~/.skillbox/
├── config.json     # 仓库目录、Agent 路径覆盖、禁用列表、主题
├── skills.json     # 已收录 Skill 索引
├── skills/         # Skill 正本
└── tmp/            # GitHub 导入时的临时解包目录
```

## 目录结构

```
.
├── index.html
├── src/                    # React 前端
│   ├── api.ts              # 与 Rust 后端的 invoke 封装
│   ├── App.tsx
│   ├── components/
│   │   ├── SkillsPage.tsx  # 技能库
│   │   ├── ImportPage.tsx  # 本地 / GitHub 导入
│   │   ├── AgentsPage.tsx  # Agent 管理
│   │   ├── MatrixPage.tsx  # 部署矩阵
│   │   ├── SettingsPage.tsx
│   │   └── …
│   └── styles.css
├── src-tauri/              # Rust 后端
│   ├── src/
│   │   ├── lib.rs          # Tauri 命令
│   │   ├── agents.rs       # Agent 定义与目录探测
│   │   ├── source.rs       # 本地 / GitHub 导入
│   │   ├── deploy.rs       # 部署 / 卸载 / 状态比对
│   │   ├── frontmatter.rs  # SKILL.md 解析
│   │   ├── fsutil.rs       # 复制、哈希、时间
│   │   └── store.rs        # 本地仓库读写
│   ├── capabilities/
│   └── tauri.conf.json
├── scripts/
│   ├── make_icon.py
│   └── setup-windows.ps1
└── .github/workflows/release.yml
```

## 设计取舍

- **复制而非符号链接**：兼容性最好，Windows 不需要开发者模式，各 Agent 副本互不干扰，
  卸载时也不会误伤源文件。代价是改了正本后需要「一键同步」推送一次。
- **文件内容哈希**：用于判断副本是否过期，比单纯看 mtime 可靠。
- **不使用 YAML 依赖**：`SKILL.md` 的 frontmatter 只用 `key: value` 和块标量，
  自写极简解析器即可，构建更快。
