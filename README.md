<div align="center">

# OmnySSH

### 你管理的每一台服务器，都在一个窗口里。仪表盘、终端、SFTP、命令片段。

<img src="assets/gui.webp" alt="OmnySSH GUI dashboard" width="900">

[![Downloads](https://img.shields.io/github/downloads/timhartmann7/omnyssh/total?label=total%20installs&color=2ea44f)](https://github.com/timhartmann7/omnyssh/releases)
[![Latest release](https://img.shields.io/github/v/release/timhartmann7/omnyssh?label=latest)](https://github.com/timhartmann7/omnyssh/releases/latest)
[![Stars](https://img.shields.io/github/stars/timhartmann7/omnyssh?style=flat)](https://github.com/timhartmann7/omnyssh/stargazers)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Build](https://img.shields.io/github/actions/workflow/status/timhartmann7/omnyssh/ci.yml?branch=main)](https://github.com/timhartmann7/omnyssh/actions)

**[安装](#安装)** •
**[国内加速下载](#国内加速下载)** •
**[功能](#它能做什么)** •
**[SSH 密钥](#ssh-密钥配置)** •
**[对比](#对比)** •
**[TUI 版本](#tui-版本)** •
**[常见问题](#常见问题)**

</div>

---

## 为什么是 OmnySSH

一款为中文用户完整本地化的开源 SSH 客户端：

- **简体中文界面**：桌面版和终端版的每个界面、对话框、错误信息都已翻译，设置里一键切换，无需任何插件。
- **免费开源**：Apache 2.0 协议，代码全部公开，没有高级版、没有订阅。
- **无需账号**：不注册、不登录、不上传任何数据，零遥测。
- **轻量**：多会话同时打开约 130 MB 内存，下载体积仅 20 MB，4 个进程。
- **全平台**：macOS（Apple Silicon / Intel）、Windows、Linux，TUI 版还支持 Termux（安卓手机直接当运维终端用）。

## 安装

在 macOS 和 Linux 上只需一条命令：

```bash
curl -fsSL https://raw.githubusercontent.com/timhartmann7/omnyssh/main/install.sh | sh
```

脚本会自动检测你的操作系统和架构，并安装最新的桌面版——macOS 上装到 `/Applications`，Linux 上加入应用菜单。想要终端版，还是两个都要？`curl … | sh -s -- --tui`（或 `--both`）。也可以直接从 [**Releases**](https://github.com/timhartmann7/omnyssh/releases/latest) 下载你平台对应的文件。

| 平台 | 文件 |
|------|------|
| macOS Apple Silicon | `OmnySSH-aarch64-apple-darwin.dmg` |
| macOS Intel | `OmnySSH-x86_64-apple-darwin.dmg` |
| Linux x86_64 | `OmnySSH-x86_64.AppImage` / `.deb` / `.rpm` |
| Windows x86_64 | `OmnySSH-x86_64-setup.exe` |

无需账号，没有登录页，零遥测。应用打开时就是一个空仪表盘，如果你已有 `~/.ssh/config`，它会自动读取——包括经由堡垒机（`ProxyJump`）连接的主机。Windows 用户常见的 Xshell、FinalShell 里保存的会话，按主机逐条加进来即可，数据不会离开你的电脑。

## 国内加速下载

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

## 它能做什么

服务器只需添加一次。之后它就以卡片的形式出现在仪表盘上，实时显示 CPU、内存和磁盘用量、运行时长、发行版、占用 CPU 最高的进程，以及一枚标明其用途的徽章。点一下 `sh` 就进入真正的 PTY 终端；点一下 `files` 就打开双栏 SFTP 浏览器。十台服务器也能一屏放下，并且自动刷新。

### 实时仪表盘
每台主机一张卡片，包含 CPU、内存和磁盘进度条、运行时长、操作系统版本、占用资源最高的进程，以及显示在线容器数量的 Docker 徽章。进度条会从黄变红，一眼就能看出哪台服务器不对劲。可以按标签分组、排序，桌面版里还能拖拽卡片排成你自己的顺序。管一堆国内云厂商（阿里云、腾讯云、华为云等）的 ECS/CVM/轻量服务器，和管任何 Linux 主机没有任何区别——标准 SSH 协议，直连即可。

### 真正的终端
以标签页形式提供完整的 PTY 会话。想开多少台服务器就开多少台，从侧边栏随意切换，它们会在你浏览仪表盘时继续运行。Vim、htop、systemctl 等交互程序都能正常使用。

### 双栏 SFTP
左边本地，右边远程。勾选想传输的文件和文件夹后一键移动，或者直接从一侧拖到另一侧，甚至可以从你的文件管理器直接拖到服务器上。有进度条可看，支持批量选择，还能看到每个文件的最后修改时间。改个 Nginx 配置、传个 jar 包，不用再记 `scp -r` 的语法。

### 命令片段
把每周都要粘贴的命令存下来。选中一个片段，勾选要执行的主机，它就会在所有主机上同时运行。片段还支持参数，比如 `sudo systemctl restart {{service}}` 会询问你服务名。批量重启几十台机器上的同一个服务，一次搞定。

### 全局搜索
按下 ⌘K 开始输入。你的所有主机，加上所有已打开的会话，都能搜到。回车即进入所选主机的终端，或回到你上次离开的会话。

### 主播模式
把屏幕上的所有真实 IP 换成假 IP。录演示视频、直播运维或共享屏幕时，不会泄露客户的公网 IP 和服务器信息。

### 亮色与暗色主题
两款应用都内置。从侧边栏即可切换。

### 英文与简体中文
两款应用的每个界面、对话框和错误信息都有简体中文翻译。在设置的主题旁边选择语言，或用 `--lang zh-Hans` 启动终端版。选择会保存到共享配置中，TUI 和桌面版会互相跟随，重启后依然生效。

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

[![Crates.io](https://img.shields.io/crates/v/omnyssh.svg)](https://crates.io/crates/omnyssh)
[![Crates downloads](https://img.shields.io/crates/d/omnyssh.svg)](https://crates.io/crates/omnyssh)

```bash
# cargo（国内建议先配置 rsproxy 镜像，见上文）
cargo install omnyssh

# homebrew
brew install timhartmann7/tap/omnyssh

# nix
nix run github:timhartmann7/omnyssh
```

然后运行 `omny`。按 `a` 添加主机，`/` 搜索，`g` 按标签分组，`?` 查看帮助，`Shift+K` 为选中的主机配置密钥。

Linux、macOS、Windows 和 Termux 的预编译 TUI 二进制文件在 [Releases](https://github.com/timhartmann7/omnyssh/releases) 页面，文件名以 `omny-*` 开头。配置位于：Linux 的 `~/.config/omnyssh/`、macOS 的 `~/Library/Application Support/omnyssh/`、Windows 的 `%APPDATA%\omnyssh\`。启动时会读取你的 `~/.ssh/config`，但绝不写入。

全部选项、快捷键和配置示例见 `man omny`。

## 常见问题

**macOS：访问局域网内的主机报 "No route to host"。** macOS 要求你明确授权后，应用才能访问局域网内的设备。在 系统设置 → 隐私与安全性 → 本地网络 中打开 OmnySSH 的开关（如果已打开，先关再开），然后重新打开应用。对于终端版 `omny`，它所运行的终端应用也需要同样的权限；Apple 自带的 Terminal 已经具备。这一条在管理家里 NAS（群晖、威联通等）时最常遇到。

**群晖（Synology）：输入密码后终端立刻显示 "Permission denied, please try again."。** DSM 只向管理员组成员提供 shell，且仅在 控制面板 → 终端机和 SNMP → 启用 SSH 服务 打开时可用。其他账号仍可使用 SFTP。

**国内服务器连不上，是不是网络被限制了？** OmnySSH 走的是标准 SSH（默认 22 端口）。连不上时先检查：云厂商安全组/防火墙有没有放行端口、控制台里的密钥对是不是和你本地的对不上、`/etc/hosts.deny` 和 `fail2ban` 有没有把你的 IP 拦了。OmnySSH 本身不做任何中转，流量直连你的服务器。

## 开发动态

作者在 Telegram 上记录正在构建的东西：发版说明、进行中的截图、性能测试，以及过程中踩过的坑。通常比其他任何地方都早。（Telegram 需要相应的网络环境才能访问。）

### 👉 [**t.me/timhartmanndev**](https://t.me/timhartmanndev)

## 参与贡献

欢迎提交 Pull Request。[CONTRIBUTING.md](CONTRIBUTING.md) 包含环境搭建、项目约定和检查清单。如果你打算做比较大的改动，请先开一个 issue，免得我们俩都在写同一个东西。发现中文翻译有问题、或想补充其他语种，也欢迎直接提 issue。

Workspace 结构：

```
crates/omnyssh-core   核心引擎，与前端无关
crates/omnyssh        TUI 应用（二进制名：omny）
crates/omnyssh-gui    Tauri 桌面应用
```

## 许可证

Apache 2.0，详见 [LICENSE](LICENSE)。

<div align="center">

### 如果 OmnySSH 帮你省下了一个终端标签页，就给仓库点个 ⭐ 吧

[报告 Bug](https://github.com/timhartmann7/omnyssh/issues) •
[功能建议](https://github.com/timhartmann7/omnyssh/issues) •
[讨论区](https://github.com/timhartmann7/omnyssh/discussions)

</div>
