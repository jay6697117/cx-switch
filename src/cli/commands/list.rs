use anyhow::Result;
use crate::core::{registry, sessions};
use crate::tui::table;

/// 执行 list 命令
pub fn execute() -> Result<()> {
    let codex_home = registry::resolve_codex_home()?;
    let mut reg = registry::load_registry(&codex_home)?;

    if reg.accounts.is_empty() {
        println!("暂无已管理的账号。");
        println!("使用 `cx-switch login` 添加当前账号，或 `cx-switch import <path>` 导入。");
        return Ok(());
    }

    // 同步活跃账号
    let _synced = registry::sync_active_account_from_auth(&codex_home, &mut reg)?;

    // 从认证文件刷新 plan 信息
    registry::refresh_accounts_from_auth(&codex_home, &mut reg)?;

    // 扫描会话日志更新额度
    if let Some(active) = reg.active_email.clone() {
        if let Some(snapshot) = sessions::scan_latest_usage(&codex_home)? {
            registry::update_usage(&mut reg, &active, snapshot);
        }
    }

    // 保存更新后的注册表
    registry::save_registry(&codex_home, &mut reg)?;

    // 打印增强表格
    table::print_accounts_table(&reg)?;

    Ok(())
}
