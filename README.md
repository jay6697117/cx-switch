# CX-Switch

> 🔄 本地 Codex 多账号管理与切换工具（Rust 实现）

## 简介

CX-Switch 是一个**纯本地**的 Codex 多账号切换 CLI 工具，通过管理 `~/.codex/auth.json` 实现账号的快速切换。

**核心特性**：
- 🔒 **纯本地操作** — 不调用任何 OpenAI API，零封号风险
- 🎨 **增强 TUI** — 颜色分级、进度条、Unicode 图标、交互式选择
- 📊 **额度监控** — 实时仪表盘、阈值告警、自动切换
- 🔄 **完全兼容** — 与原始 `codex-auth` 工具的 `registry.json` 双向兼容

## 命令一览

| 命令 | 说明 |
|------|------|
| `cx-switch list` | 列出所有账号（邮箱、计划、额度、最后活跃） |
| `cx-switch login` | 通过openai oauth登录账号 |
| `cx-switch login [--skip]` | 登录并添加当前账号 |
| `cx-switch switch [email]` | 交互式 / 模糊匹配切换账号 |
| `cx-switch import <path> [--alias name]` | 导入认证文件或目录 |
| `cx-switch remove` | 交互式多选删除账号 |
| `cx-switch watch [--interval 60] [--threshold 20] [--auto-switch]` | 额度监控仪表盘 |

## 环境要求

- **Rust** ≥ 1.70（推荐使用 [rustup](https://rustup.rs/) 安装）
- **Codex CLI** 已安装且 `~/.codex/` 目录存在

## 本地开发

```bash
# 克隆项目
git clone <repo-url>
cd cx-switch

# 开发编译（快速，含调试信息）
cargo build

# 运行（开发模式）
cargo run -- list                              # 列出所有账号
cargo run -- login                             # 通过openai oauth登录账号
cargo run -- login --skip                      # 导入当前 auth.json
cargo run -- switch                            # 交互式切换账号
cargo run -- switch user1                      # 模糊匹配切换
cargo run -- import ./auth.json --alias test   # 导入认证文件
cargo run -- remove                            # 交互式删除账号
cargo run -- watch --interval 30               # 额度监控仪表盘
cargo run -- --help                            # 查看帮助
cargo run -- --version                         # 查看版本

# 也可以直接运行编译产物
./target/debug/cx-switch list
```

## 测试

```bash
# 运行全部单元测试
cargo test

# 运行指定模块的测试
cargo test core::auth
cargo test core::models
cargo test core::sessions
cargo test utils::timefmt

# 查看测试输出
cargo test -- --nocapture
```

## 打包发布

```bash
# Release 编译（优化，体积小，速度快）
cargo build --release

# 编译产物位于
./target/release/cx-switch

# 安装到系统 PATH（~/.cargo/bin/）
cargo install --path .

# 安装后直接使用
cx-switch --version
cx-switch list
```

### 交叉编译（可选）

```bash
# 安装交叉编译目标
rustup target add x86_64-unknown-linux-gnu
rustup target add x86_64-pc-windows-gnu
rustup target add aarch64-apple-darwin

# 编译指定平台
cargo build --release --target x86_64-unknown-linux-gnu
cargo build --release --target aarch64-apple-darwin
```

## 项目结构

```
cx-switch/
├── Cargo.toml                # 项目配置和依赖
├── src/
│   ├── main.rs               # 入口 + 命令分发
│   ├── cli/
│   │   ├── mod.rs            # clap 命令定义
│   │   └── commands/         # 各子命令处理器
│   │       ├── list.rs       # list 命令
│   │       ├── login.rs      # login 命令
│   │       ├── switch_cmd.rs # switch 命令
│   │       ├── import.rs     # import 命令
│   │       ├── remove.rs     # remove 命令
│   │       └── watch.rs      # watch 守护进程
│   ├── core/
│   │   ├── models.rs         # 数据类型（AccountRecord, Registry 等）
│   │   ├── auth.rs           # JWT 解析、auth.json 读取
│   │   ├── registry.rs       # 注册表 CRUD、备份、导入
│   │   └── sessions.rs       # 会话日志扫描、额度提取
│   ├── tui/
│   │   ├── theme.rs          # 颜色主题、进度条
│   │   ├── icons.rs          # Unicode 图标常量
│   │   ├── table.rs          # 增强表格渲染
│   │   ├── selector.rs       # 交互式单选组件
│   │   ├── multi_selector.rs # 交互式多选组件
│   │   └── dashboard.rs      # watch 仪表盘 TUI
│   └── utils/
│       └── timefmt.rs        # 相对时间格式化
└── codex-auth-origin/        # 原始 Zig 项目（参考实现）
```

## 依赖说明

| crate | 用途 |
|-------|------|
| `clap` 4.x | 命令行参数解析 |
| `serde` + `serde_json` | JSON 序列化 / 反序列化 |
| `base64` | JWT payload 解码 |
| `crossterm` | 终端控制（raw mode、颜色、光标） |
| `chrono` | 本地时间格式化 |
| `dirs` | 跨平台 HOME 目录获取 |
| `anyhow` | 错误处理 |
| `terminal_size` | 终端宽度检测 |
| `unicode-width` | Unicode 字符宽度计算 |

## 数据文件

| 路径 | 说明 |
|------|------|
| `~/.codex/auth.json` | 当前活跃账号的认证文件 |
| `~/.codex/accounts/registry.json` | 注册表（所有账号元数据） |
| `~/.codex/accounts/<base64url>.auth.json` | 各账号的认证文件 |
| `~/.codex/sessions/rollout-*.jsonl` | 会话日志（用于提取额度数据） |

## 许可证

MIT