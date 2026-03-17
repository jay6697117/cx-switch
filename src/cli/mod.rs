pub mod commands;

use clap::{Parser, Subcommand};

/// 本地 Codex 多账号切换工具
#[derive(Parser)]
#[command(
    name = "cx-switch",
    version,
    about = "🔄 本地 Codex 多账号管理与切换工具",
    long_about = "🔄 CX-Switch — 本地 Codex 多账号管理与切换工具（Rust 实现）\n\n\
        核心特性：\n  \
        🔒 纯本地操作 — 不调用任何 OpenAI API，零封号风险\n  \
        🎨 增强 TUI  — 颜色分级、进度条、Unicode 图标、交互式选择\n  \
        📊 额度监控  — 实时仪表盘、阈值告警、自动切换\n  \
        🔄 完全兼容  — 与原始 codex-auth 工具的 registry.json 双向兼容\n\n\
        文档: https://github.com/jay6697117/cx-switch"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// 列出所有账号（邮箱、计划、额度、最后活跃）
    List,

    /// 登录并添加当前账号
    #[command(
        long_about = "登录并添加当前账号\n\n\
            默认行为：调用 codex login 进行 OpenAI OAuth 登录，登录成功后自动将账号添加到本地注册表。\n\n\
            使用 --skip 跳过 OAuth 登录，直接读取本地已有的 ~/.codex/auth.json 文件并导入。"
    )]
    Login {
        /// 跳过 codex login，直接读取本地 auth.json 导入
        #[arg(long)]
        skip: bool,
    },

    /// 交互式 / 模糊匹配切换活跃账号
    #[command(
        long_about = "切换活跃账号\n\n\
            不带参数：进入交互式选择界面，用方向键选择账号。\n\
            带邮箱参数：模糊匹配邮箱，直接切换到匹配的账号。\n\n\
            示例：\n  \
            cx-switch switch            # 交互式选择\n  \
            cx-switch switch user1      # 模糊匹配切换"
    )]
    Switch {
        /// 邮箱或邮箱片段（模糊匹配，留空进入交互式选择）
        email: Option<String>,
    },

    /// 导入认证文件或目录
    #[command(
        long_about = "导入认证文件或目录\n\n\
            支持导入单个 auth.json 文件或包含多个认证文件的目录。\n\
            可通过 --alias 为导入的账号设置别名。\n\n\
            示例：\n  \
            cx-switch import ./auth.json\n  \
            cx-switch import ./auth.json --alias test\n  \
            cx-switch import ./accounts/"
    )]
    Import {
        /// 认证文件路径或目录路径
        path: String,

        /// 为导入的账号设置别名
        #[arg(long)]
        alias: Option<String>,
    },

    /// 交互式多选删除账号
    Remove,

    /// 额度监控实时仪表盘
    #[command(
        long_about = "额度监控实时仪表盘\n\n\
            启动实时仪表盘，定时刷新显示所有账号的额度信息。\n\
            支持设置低额度阈值告警，以及额度不足时自动切换到最佳账号。\n\n\
            模式：\n  \
            默认启动终端 TUI 仪表盘\n  \
            --web 启动 Web 仪表盘，浏览器访问\n\n\
            示例：\n  \
            cx-switch watch                              # 终端 TUI 仪表盘\n  \
            cx-switch watch --web                        # Web 仪表盘 (localhost:9394)\n  \
            cx-switch watch --web --port 8080            # 自定义端口\n  \
            cx-switch watch --interval 30                # 30 秒刷新\n  \
            cx-switch watch --threshold 10 --auto-switch # 低于 10% 自动切换"
    )]
    Watch {
        /// 检查间隔（秒，默认 60）
        #[arg(long, default_value = "60")]
        interval: u64,

        /// 低额度阈值（百分比，默认 20）
        #[arg(long, default_value = "20")]
        threshold: u64,

        /// 额度不足时自动切换到最佳账号
        #[arg(long)]
        auto_switch: bool,

        /// 启用 Web 仪表盘模式（浏览器访问）
        #[arg(long)]
        web: bool,

        /// Web 服务器端口
        #[arg(long, default_value = "9394")]
        port: u16,
    },

    /// 登录并添加当前账号（废弃别名，请使用 login）
    #[command(hide = true)]
    Add {
        /// 跳过 codex login
        #[arg(long)]
        skip: bool,

        /// 废弃参数，等同于 --skip
        #[arg(long, hide = true)]
        no_login: bool,
    },
}

