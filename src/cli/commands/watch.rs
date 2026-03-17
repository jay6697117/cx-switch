use anyhow::Result;
use crate::core::registry;
use crate::tui::{dashboard, web};

/// 执行 watch 命令
pub fn execute(interval: u64, threshold: u64, auto_switch: bool, web_mode: bool, port: u16) -> Result<()> {
    let codex_home = registry::resolve_codex_home()?;
    let reg = registry::load_registry(&codex_home)?;

    if reg.accounts.is_empty() {
        println!("暂无已管理的账号。");
        println!("使用 `cx-switch login` 添加当前账号。");
        return Ok(());
    }

    if web_mode {
        // Web 仪表盘模式
        web::run_web_dashboard(&codex_home, interval, threshold, auto_switch, port)?;
    } else {
        // 终端 TUI 仪表盘模式
        println!("启动额度监控仪表盘...");
        println!("  间隔: {}s · 阈值: {}% · 自动切换: {}", interval, threshold, auto_switch);
        println!("  按 Ctrl+C 或 q 退出");
        println!();
        dashboard::run_dashboard(&codex_home, interval, threshold, auto_switch)?;
    }

    Ok(())
}
