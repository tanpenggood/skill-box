# SkillBox · Windows 开发环境准备脚本
# 用法：右键「使用 PowerShell 运行」，或在管理员 PowerShell 中执行
#   Set-ExecutionPolicy -Scope Process Bypass; .\scripts\setup-windows.ps1

$ErrorActionPreference = 'Continue'

function Write-Step($msg) { Write-Host "`n=== $msg ===" -ForegroundColor Cyan }
function Write-Ok($msg)   { Write-Host "  [OK] $msg" -ForegroundColor Green }
function Write-Warn($msg) { Write-Host "  [!]  $msg" -ForegroundColor Yellow }
function Write-Bad($msg)  { Write-Host "  [X]  $msg" -ForegroundColor Red }

Write-Step "1/4 检查 Node.js"
if (Get-Command node -ErrorAction SilentlyContinue) {
    Write-Ok ("Node " + (node -v))
} else {
    Write-Bad "未检测到 Node.js，请到 https://nodejs.org 安装 LTS 版本后重新运行本脚本"
    exit 1
}

Write-Step "2/4 检查 Rust"
$cargo = Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe'
if (Get-Command cargo -ErrorAction SilentlyContinue) {
    Write-Ok ("cargo " + (cargo --version))
} elseif (Test-Path $cargo) {
    $env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
    Write-Ok ("cargo 已安装在用户目录：" + (& $cargo --version))
} else {
    Write-Warn "未检测到 Rust，正在使用国内镜像安装 rustup…"
    $env:RUSTUP_DIST_SERVER = 'https://rsproxy.cn'
    $env:RUSTUP_UPDATE_ROOT = 'https://rsproxy.cn/rustup'
    $installer = Join-Path $env:TEMP 'rustup-init.exe'
    Invoke-WebRequest -Uri 'https://rsproxy.cn/rustup/dist/x86_64-pc-windows-msvc/rustup-init.exe' -OutFile $installer -UseBasicParsing
    & $installer -y --default-toolchain stable --profile minimal --no-modify-path
    $env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
    Write-Ok "Rust 安装完成"
}

Write-Step "3/4 检查 MSVC C++ 生成工具（Tauri 在 Windows 上必须使用 MSVC 链接器）"
$linkFound = Get-ChildItem 'C:\Program Files\Microsoft Visual Studio' -Filter 'link.exe' -Recurse -ErrorAction SilentlyContinue |
    Where-Object { $_.FullName -like '*Hostx64\x64*' } | Select-Object -First 1

if ($linkFound) {
    Write-Ok ("已找到链接器：" + $linkFound.FullName)
} else {
    Write-Warn "未找到 MSVC 链接器。SkillBox 需要 Visual Studio 的「使用 C++ 的桌面开发」工作负载。"
    Write-Host ""
    Write-Host "  请二选一：" -ForegroundColor Yellow
    Write-Host "  A) 自动安装（需要管理员权限，约 3GB）：" -ForegroundColor Yellow
    Write-Host "     winget install Microsoft.VisualStudio.2022.BuildTools --override `"--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended`"" -ForegroundColor White
    Write-Host "  B) 若已安装 VS Community，给它补装 C++ 工作负载：" -ForegroundColor Yellow
    Write-Host "     打开 Visual Studio Installer → 修改 → 勾选「使用 C++ 的桌面开发」→ 安装" -ForegroundColor White
    Write-Host ""
    $ans = Read-Host "  是否现在尝试自动安装 Build Tools？(y/N)"
    if ($ans -eq 'y' -or $ans -eq 'Y') {
        if (Get-Command winget -ErrorAction SilentlyContinue) {
            winget install Microsoft.VisualStudio.2022.BuildTools --override "--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
        } else {
            Write-Bad "本机没有 winget，请手动到 https://visualstudio.microsoft.com/visual-cpp-build-tools/ 下载安装"
        }
    }
}

Write-Step "4/4 安装前端依赖"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root
if (-not (Test-Path 'node_modules')) {
    npm install
} else {
    Write-Ok "node_modules 已存在"
}

Write-Host "`n准备完成。接下来可以运行：" -ForegroundColor Green
Write-Host "  npm run tauri:dev     开发模式启动" -ForegroundColor White
Write-Host "  npm run tauri:build   打包 Windows 安装包" -ForegroundColor White
