<div align="center">

# OmnySSH 简体中文版

### 现代化 TUI / GUI SSH 服务器管理工具 —— 仪表盘、终端、SFTP、命令片段全集成

<img src="assets/gui.webp" alt="OmnySSH GUI dashboard" width="900">

[![Downloads](https://img.shields.io/github/downloads/WuYiLingOps/omnyssh/total?label=total%20installs&color=2ea44f)](https://github.com/WuYiLingOps/omnyssh/releases)
[![Latest release](https://img.shields.io/github/v/release/WuYiLingOps/omnyssh?label=latest)](https://github.com/WuYiLingOps/omnyssh/releases/latest)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Build](https://img.shields.io/github/actions/workflow/status/WuYiLingOps/omnyssh/ci.yml?branch=main)](https://github.com/WuYiLingOps/omnyssh/actions)

**[项目简介](#项目简介)** •
**[功能特性](#主要功能特性)** •
**[安装](#安装与使用)** •
**[国内加速](#国内加速下载)** •
**[SSH 密钥](#ssh-密钥配置)** •
**[对比](#对比)** •
**[TUI 版本](#tui-版本)** •
**[常见问题](#常见问题)** •
**[致谢](#致敬原作者与开源协议)**

</div>

---

## 项目简介

**OmnySSH 简体中文版**是基于官方上游 [timhartmann7/omnyssh](https://github.com/timhartmann7/omnyssh) 深度定制的中文本地化分支。

本项目引入了轻量级、无侵入的**多语言国际化（i18n）架构**，为桌面版和终端版的每个界面、对话框和错误信息提供全套简体中文（zh-Hans）翻译，在设置中一键切换语言，无需任何插件。

同时保留了上游项目的全部优点：

- **免费开源**：Apache 2.0 协议，代码全部公开，没有高级版、没有订阅。
- **无需账号**：不注册、不登录、不上传任何数据，零遥测。
- **轻量**：多会话同时打开约 130 MB 内存，下载体积仅 20 MB，4 个进程。
- **全平台**：macOS（Apple Silicon / Intel）、Windows、Linux，TUI 版还支持 Termux（安卓手机直接当运维终端用）。

## 主要功能特性

服务器只需添加一次。之后它就以卡片的形式出现在仪表盘上，实时显示 CPU、内存和磁盘用量、运行时长、发行版、占用 CPU 最高的进程，以及一枚标明其用途的徽章。点一下 `sh` 就进入真正的 PTY 终端；点一下 `files` 就打开双栏 SFTP 浏览器。十台服务器也能一屏放下，并且自动刷新。管一堆国内云厂商（阿里云、腾讯云、华为云等）的 ECS/CVM/轻量服务器，和管任何 Linux 主机没有任何区别——标准 SSH 协议，直连即可。

- **实时服务器仪表盘**：单窗口统一监控多台 VPS/服务器，直观展示 CPU、内存、磁盘使用率进度条、运行时长、操作系统及 Docker 等服务运行状态，超载时进度条由黄变红；支持按标签分组、排序，桌面版可拖拽卡片自定义布局。
- **内置多标签终端 (PTY)**：基于 xterm 的高性能全功能终端，支持分屏会话与后台无缝切换，Vim、htop、systemctl 等交互程序正常使用。
- **双栏式 SFTP 文件传输**：左侧本地、右侧远程，支持批量上传、下载、新建目录、重命名与拖拽传输，可从文件管理器直接拖到服务器，告别繁琐的 `scp` 命令。
- **命令片段与多机广播**：保存高频运维命令，支持动态参数插值（如 `sudo systemctl restart {{service}}`），一键向选中的多台服务器并发广播执行，批量重启几十台机器上的同一个服务一次搞定。
- **自动化 SSH 密钥免密配置**：一键生成 Ed25519 密钥、分发公钥、验证连接并安全禁用密码认证，全程自动回滚保障服务器安全。
- **⌘K 全局快速命令面板**：键盘驱动，快速搜索与切换主机和活动会话。
- **主播/隐私模式 (Streamer Mode)**：一键对界面中所有敏感主机名与 IP 地址脱敏，适合录屏与直播演示。
- **亮色与暗色主题**：两款应用都内置，从侧边栏即可切换。
- **多语言随时切换**：在“设置 (Settings)”中自由切换 **简体中文** 与 **English**，选择保存在共享配置中，TUI 与桌面版互相跟随，重启后依然生效。

## 安装与使用

### GUI 桌面客户端

前往 [**Releases 发行版页面**](https://github.com/WuYiLingOps/omnyssh/releases) 下载对应平台的安装包：

| 操作系统平台 | 文件格式 | 说明 |
|---|---|---|
| **Windows x86_64** | `OmnySSH-x86_64-setup.exe` / `.zip` | 适用于 64 位 Windows 系统（安装版 / 便携版） |
| **macOS (Apple Silicon)** | `OmnySSH-aarch64-apple-darwin.dmg` | 适用于 M1/M2/M3/M4 系列芯片 |
| **macOS (Intel)** | `OmnySSH-x86_64-apple-darwin.dmg` | 适用于 Intel 处理器 Mac |
| **Linux x86_64** | `OmnySSH-x86_64.AppImage` / `.deb` / `.rpm` | 通用 Linux 发行版 |

> **无账号、无遥测、无数据上传**：应用本地运行，自动读取你的 `~/.ssh/config` 配置（包括经由堡垒机 `ProxyJump` 连接的主机），隐私安全。Windows 用户在 Xshell、FinalShell 里保存的会话，按主机逐条加进来即可，数据不会离开你的电脑。

### 国内加速下载

如果你的网络访问 GitHub 较慢，以下方式可选（均为社区通行做法，请自行判断可信度）：

- **Release 文件加速**：用 `ghproxy` 类镜像站前缀拼接下载链接，或让终端走代理（`export https_proxy=http://127.0.0.1:7890`）后再执行安装脚本。
- **cargo 安装 TUI 版**：推荐配置 [rsproxy（字节跳动镜像）](https://rsproxy.cn)：

  ```toml
  # ~/.cargo/config.toml
  [source.crates-io]
  replace-with = 'rsproxy-sparse'
  [source.rsproxy-sparse]
  registry = "sparse+https://rsproxy.cn/index/"
  [registries.rsproxy]
  index = "sparse+https://rsproxy.cn/index/"
  ```

- **应用内更新**：OmnySSH 本身不联网上报任何数据，检查更新同样只访问 GitHub Releases，走系统代理即可。

## SSH 密钥配置

新买的 VPS 还在用密码登录，就是你总想着要改却永远不会动手的那件事。OmnySSH 一次点击搞定。

选一台你自己添加、尚未配置密钥的主机，点 **Set up SSH key**，应用会生成一个 Ed25519 密钥，把公钥追加到 `authorized_keys`，然后将主机切换为密钥认证。接着它会用新密钥发起一条全新连接来验证密钥可用，验证通过后才会关闭密码登录。中间没有确认步骤：一旦开始就会走完全程。

在改动 `sshd_config` 之前，它会在服务器上保存一份备份。任何一步失败，它都会恢复备份，让你的访问方式保持原样。你的私钥永远不会离开你的机器，除了你选定的服务器之外，什么都不会发送到任何地方。

代码在 [`crates/omnyssh-core/src/ssh/key_setup.rs`](crates/omnyssh-core/src/ssh/key_setup.rs)。在把它指向生产环境之前请先读一读——这正是把它开源的意义所在。

带口令（passphrase）的密钥也没问题。密钥被加密时，OmnySSH 会询问一次口令并保存在内存中，直到你退出应用；它从不写入磁盘。在 macOS 和 Linux 上，已加载到 `ssh-agent` 的密钥会直接使用，无需再次询问。主机的 **Password** 字段是服务器登录密码，不是密钥口令。

## 对比

内存和 CPU 数据在一台 M4 Mac 上测得，两个应用均处于打开且空闲状态。

| | OmnySSH | Termius | tmux + ssh |
|---|---|---|---|
| 空闲内存 | ~130 MB | ~649 MB | 极小 |
| 进程数 | 4 | 9 | 1 |
| 实时指标仪表盘 | ✅ | ✅ | ❌ |
| 双栏 SFTP | ✅ | ✅ | ❌ |
| 片段与批量执行 | ✅ | ✅ | ❌ |
| 一键密钥配置 | ✅ | ❌ | ❌ |
| 需要账号 | ❌ | ✅ | ❌ |
| 遥测 | ❌ | ✅ | ❌ |
| 简体中文界面 | ✅ | ✅ | ❌ |
| 开源 | ✅ | ❌ | ✅ |
| 价格 | 免费 | 💰 | 免费 |

tmux 之所以留在表里，是因为我们大多数人实际在用的就是它。它赢在轻量，输在所有看得见的地方。

## TUI 版本

OmnySSH 起源于终端，TUI 版本至今仍在维护，仍在发版。习惯命令行、或者想在安卓手机上用 Termux 运维的同学，这版就是为你准备的。

![Demo](assets/demo.gif)

底层是同一个引擎：本仓库是一个 cargo workspace，`crates/omnyssh-core` 承载核心逻辑，各前端构建于其上。仪表盘、SFTP、命令片段、多会话标签、模糊搜索，外加四套主题（`default`、`dracula`、`nord`、`gruvbox`）以及可在 `config.toml` 中重新映射的快捷键。界面提供英文和简体中文——在设置中选择，或用 `--lang zh-Hans` 启动。

从 [Releases](https://github.com/WuYiLingOps/omnyssh/releases) 下载预编译的 TUI 二进制文件（文件名以 `omny-*` 开头，覆盖 Linux、macOS、Windows 和 Termux），或直接用 cargo 从本仓库安装：

```bash
cargo install --git https://github.com/WuYiLingOps/omnyssh omnyssh --locked
```

然后运行 `omny`。按 `a` 添加主机，`/` 搜索，`g` 按标签分组，`?` 查看帮助，`Shift+K` 为选中的主机配置密钥。

配置位于：Linux 的 `~/.config/omnyssh/`、macOS 的 `~/Library/Application Support/omnyssh/`、Windows 的 `%APPDATA%\omnyssh\`。启动时会读取你的 `~/.ssh/config`，但绝不写入。

全部选项、快捷键和配置示例见 `man omny`。

## 跟随上游更新

本项目持续跟踪官方上游 [timhartmann7/omnyssh](https://github.com/timhartmann7/omnyssh) 的更新，上游新特性会及时合入并完成汉化：

1. **上游特性全保留**：仪表盘、SFTP、命令片段、密钥配置等核心功能与上游保持同步演进。
2. **语言包彻底隔离**：桌面版翻译字典独立存放在 `crates/omnyssh-gui/ui/src/lib/i18n/locales/`（`zh-Hans.json` / `en.json`），TUI 词典位于 `crates/omnyssh-core/src/i18n.rs`，与核心业务代码分离，合并上游时冲突率极低。

## 常见问题

**macOS：访问局域网内的主机报 "No route to host"。** macOS 要求你明确授权后，应用才能访问局域网内的设备。在 系统设置 → 隐私与安全性 → 本地网络 中打开 OmnySSH 的开关（如果已打开，先关再开），然后重新打开应用。对于终端版 `omny`，它所运行的终端应用也需要同样的权限；Apple 自带的 Terminal 已经具备。这一条在管理家里 NAS（群晖、威联通等）时最常遇到。

**群晖（Synology）：输入密码后终端立刻显示 "Permission denied, please try again."。** DSM 只向管理员组成员提供 shell，且仅在 控制面板 → 终端机和 SNMP → 启用 SSH 服务 打开时可用。其他账号仍可使用 SFTP。

**国内服务器连不上，是不是网络被限制了？** OmnySSH 走的是标准 SSH（默认 22 端口）。连不上时先检查：云厂商安全组/防火墙有没有放行端口、控制台里的密钥对是不是和你本地的对不上、`/etc/hosts.deny` 和 `fail2ban` 有没有把你的 IP 拦了。OmnySSH 本身不做任何中转，流量直连你的服务器。

## 致敬原作者与开源协议

- **官方上游仓库**：[timhartmann7/omnyssh](https://github.com/timhartmann7/omnyssh)
- **开源协议**：[Apache License 2.0](LICENSE)
- 感谢原作者 **Tim Hartmann**（[@timhartmann7](https://github.com/timhartmann7)）打造的开源工具。

## 参与贡献

欢迎提交 Pull Request。[CONTRIBUTING.md](CONTRIBUTING.md) 包含环境搭建、项目约定和检查清单。如果你打算做比较大的改动，请先开一个 issue，免得我们俩都在写同一个东西。发现中文翻译有问题、或想补充其他语种，也欢迎直接提 issue。

Workspace 结构：

```
crates/omnyssh-core   核心引擎，与前端无关（含 i18n 词典）
crates/omnyssh        TUI 应用（二进制名：omny）
crates/omnyssh-gui    Tauri 桌面应用（含 i18n 语言包）
```

## 许可证

Apache 2.0，详见 [LICENSE](LICENSE)。

<div align="center">

### 如果 OmnySSH 帮你省下了一个终端标签页，就给仓库点个 ⭐ 吧

[报告 Bug](https://github.com/WuYiLingOps/omnyssh/issues) •
[功能建议](https://github.com/WuYiLingOps/omnyssh/issues) •
[官方项目原址](https://github.com/timhartmann7/omnyssh)

</div>
