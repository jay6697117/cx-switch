use crate::core::models::*;
use crate::core::{registry, sessions};
use crate::tui::{icons, theme};
use chrono::prelude::*;
use crossterm::{
    cursor, event,
    terminal::{self, ClearType},
    ExecutableCommand,
};
use std::io::{self, Write};
use std::time::Duration;

/// 事件日志条目
struct EventEntry {
    time: String,
    message: String,
}

/// 仪表盘运行状态
struct DashboardState {
    events: Vec<EventEntry>,
    max_events: usize,
}

impl DashboardState {
    fn new() -> Self {
        DashboardState {
            events: Vec::new(),
            max_events: 10,
        }
    }

    /// 添加事件（保留最近 max_events 条）
    fn add_event(&mut self, message: String) {
        let now = Local::now();
        self.events.push(EventEntry {
            time: now.format("%H:%M:%S").to_string(),
            message,
        });
        while self.events.len() > self.max_events {
            self.events.remove(0);
        }
    }
}

/// 运行仪表盘主循环
pub fn run_dashboard(
    codex_home: &std::path::Path,
    interval: u64,
    threshold: u64,
    auto_switch: bool,
) -> anyhow::Result<()> {
    let stdout = io::stdout();
    let mut out = stdout.lock();

    terminal::enable_raw_mode()?;
    let _cleanup = RawModeGuard;

    let mut state = DashboardState::new();
    state.add_event("仪表盘已启动".to_string());

    loop {
        // 加载最新数据
        let mut reg = registry::load_registry(codex_home)?;
        let _ = registry::sync_active_account_from_auth(codex_home, &mut reg);

        // 刷新额度
        if let Ok(Some(snapshot)) = sessions::scan_latest_usage(codex_home) {
            if let Some(active) = reg.active_email.clone() {
                registry::update_usage(&mut reg, &active, snapshot);
                // 持久化额度数据到 registry.json
                let _ = registry::save_registry(codex_home, &mut reg);
            }
        }

        // 检查阈值告警
        check_thresholds(&reg, threshold, auto_switch, codex_home, &mut state)?;

        // 如果启用自动切换且有切换事件，重新加载
        let reg = registry::load_registry(codex_home)?;

        // 渲染仪表盘
        out.execute(terminal::Clear(ClearType::All))?;
        out.execute(cursor::MoveTo(0, 0))?;
        render_dashboard(&mut out, &reg, interval, threshold, &state)?;
        out.flush()?;

        // 等待用户输入或超时
        if event::poll(Duration::from_secs(interval))? {
            if let event::Event::Key(key) = event::read()? {
                match key.code {
                    event::KeyCode::Char('c')
                        if key.modifiers.contains(event::KeyModifiers::CONTROL) =>
                    {
                        return Ok(());
                    }
                    event::KeyCode::Esc | event::KeyCode::Char('q') => {
                        return Ok(());
                    }
                    _ => {}
                }
            }
        }
    }
}

/// 检查阈值并产生告警/自动切换
fn check_thresholds(
    reg: &Registry,
    threshold: u64,
    auto_switch: bool,
    codex_home: &std::path::Path,
    state: &mut DashboardState,
) -> anyhow::Result<()> {
    let threshold = threshold as i64;

    for rec in &reg.accounts {
        let rate_5h = resolve_rate_window(&rec.last_usage, 300, true);
        let rate_week = resolve_rate_window(&rec.last_usage, 10080, false);

        // 检查 5H 额度
        if let Some(w) = rate_5h {
            let rem = remaining_percent(w.used_percent);
            if rem == 0 {
                state.add_event(format!(
                    "{} {} 5H 额度已耗尽",
                    icons::CROSS,
                    short_email(&rec.email)
                ));
            } else if rem < threshold {
                state.add_event(format!(
                    "{} {} 5H 额度低于 {}%（剩余 {}%）",
                    icons::WARN,
                    short_email(&rec.email),
                    threshold,
                    rem
                ));
            }
        }

        // 检查周额度
        if let Some(w) = rate_week {
            let rem = remaining_percent(w.used_percent);
            if rem == 0 {
                state.add_event(format!(
                    "{} {} WEEKLY 额度已耗尽",
                    icons::CROSS,
                    short_email(&rec.email)
                ));
            } else if rem < threshold {
                state.add_event(format!(
                    "{} {} WEEKLY 额度低于 {}%（剩余 {}%）",
                    icons::WARN,
                    short_email(&rec.email),
                    threshold,
                    rem
                ));
            }
        }
    }

    // 自动切换逻辑：当活跃账号 5H 额度耗尽时切换
    if auto_switch {
        if let Some(ref active) = reg.active_email {
            let active_rec = reg.accounts.iter().find(|r| &r.email == active);
            if let Some(rec) = active_rec {
                let rate_5h = resolve_rate_window(&rec.last_usage, 300, true);
                let should_switch = rate_5h
                    .map(|w| remaining_percent(w.used_percent) == 0)
                    .unwrap_or(false);

                if should_switch {
                    if let Some(best_idx) = registry::select_best_account_index_by_usage(reg) {
                        let best_email = &reg.accounts[best_idx].email;
                        if best_email != active {
                            let best_rem = {
                                let w = resolve_rate_window(
                                    &reg.accounts[best_idx].last_usage,
                                    300,
                                    true,
                                );
                                w.map(|w| remaining_percent(w.used_percent))
                                    .unwrap_or(0)
                            };

                            // 执行切换
                            let active_path = registry::active_auth_path(codex_home);
                            let new_path = registry::account_auth_path(codex_home, best_email);
                            if new_path.exists() {
                                let _ = registry::backup_auth_if_changed(
                                    codex_home,
                                    &active_path,
                                    &new_path,
                                );
                                let _ = registry::copy_file(&new_path, &active_path);
                                let mut reg_mut = registry::load_registry(codex_home)?;
                                registry::set_active_account(&mut reg_mut, best_email);
                                let _ = registry::save_registry(codex_home, &mut reg_mut);

                                state.add_event(format!(
                                    "{} 自动切换到 {}（额度最高 {}%）",
                                    icons::REFRESH,
                                    short_email(best_email),
                                    best_rem
                                ));
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

/// 渲染仪表盘
fn render_dashboard(
    out: &mut impl Write,
    reg: &Registry,
    interval: u64,
    threshold: u64,
    state: &DashboardState,
) -> io::Result<()> {
    let now = registry::now_timestamp();
    let last_check = Local::now().format("%H:%M").to_string();
    let width = theme::terminal_width().max(60);
    let inner_width = width - 4; // 减去左右边框和空格

    // 顶部边框
    write!(out, "  {}", icons::BOX_TOP_LEFT)?;
    write!(out, "{}", icons::BOX_DOUBLE_H.repeat(inner_width))?;
    writeln!(out, "{}\r", icons::BOX_TOP_RIGHT)?;

    // 标题栏
    let title = format!(
        "  cx-switch 额度监控  ⏱ 每 {}s 刷新 · 阈值 {}% · 上次 {}",
        interval, threshold, last_check
    );
    write!(out, "  {} ", icons::BOX_VERTICAL)?;
    write!(out, "{}", theme::active_style(&truncate_str(&title, inner_width - 2)))?;
    let pad = inner_width.saturating_sub(title.len() + 1);
    write!(out, "{}", " ".repeat(pad))?;
    writeln!(out, "{}\r", icons::BOX_VERTICAL)?;

    // 中间分隔线
    write!(out, "  {}", icons::BOX_T_LEFT)?;
    write!(out, "{}", icons::BOX_DOUBLE_H.repeat(inner_width))?;
    writeln!(out, "{}\r", icons::BOX_T_RIGHT)?;

    // 账号列表区域
    for rec in &reg.accounts {
        let is_active = reg.active_email.as_deref() == Some(&rec.email);
        let plan = resolve_plan(rec)
            .map(|p| p.to_string())
            .unwrap_or_else(|| "-".to_string());

        // 账号标题行
        let active_mark = if is_active { icons::ACTIVE } else { " " };
        let active_tag = if is_active { "  [ACTIVE]" } else { "" };
        let header = format!(
            "  {} {} [{}]{}",
            active_mark, rec.email, plan, active_tag
        );

        write!(out, "  {} ", icons::BOX_VERTICAL)?;
        if is_active {
            write!(out, "{}", theme::active_style(&header))?;
        } else {
            write!(out, "{}", header)?;
        }
        let pad = inner_width.saturating_sub(header.len() + 1);
        write!(out, "{}", " ".repeat(pad))?;
        writeln!(out, "{}\r", icons::BOX_VERTICAL)?;

        // 5H 额度行
        let rate_5h = resolve_rate_window(&rec.last_usage, 300, true);
        render_rate_line(out, "5H  ", rate_5h, now, threshold as i64, inner_width)?;

        // WEEK 额度行
        let rate_week = resolve_rate_window(&rec.last_usage, 10080, false);
        render_rate_line(out, "WEEK", rate_week, now, threshold as i64, inner_width)?;

        // 空行
        write!(out, "  {} ", icons::BOX_VERTICAL)?;
        write!(out, "{}", " ".repeat(inner_width - 1))?;
        writeln!(out, "{}\r", icons::BOX_VERTICAL)?;
    }

    // 事件区分隔线
    if !state.events.is_empty() {
        write!(out, "  {}", icons::BOX_T_LEFT)?;
        write!(out, "{}", icons::BOX_DOUBLE_H.repeat(inner_width))?;
        writeln!(out, "{}\r", icons::BOX_T_RIGHT)?;

        for ev in &state.events {
            let line = format!("  [事件] {} {}", ev.time, ev.message);
            write!(out, "  {} ", icons::BOX_VERTICAL)?;
            write!(out, "{}", theme::dim_style(&truncate_str(&line, inner_width - 2)))?;
            let pad = inner_width.saturating_sub(line.len() + 1);
            write!(out, "{}", " ".repeat(pad))?;
            writeln!(out, "{}\r", icons::BOX_VERTICAL)?;
        }
    }

    // 底部边框
    write!(out, "  {}", icons::BOX_BOTTOM_LEFT)?;
    write!(out, "{}", icons::BOX_DOUBLE_H.repeat(inner_width))?;
    writeln!(out, "{}\r", icons::BOX_BOTTOM_RIGHT)?;

    writeln!(
        out,
        "  {}",
        theme::dim_style("Ctrl+C 或 q 退出")
    )?;

    Ok(())
}

/// 渲染一行额度信息
fn render_rate_line(
    out: &mut impl Write,
    label: &str,
    window: Option<&RateLimitWindow>,
    now: i64,
    threshold: i64,
    inner_width: usize,
) -> io::Result<()> {
    write!(out, "  {} ", icons::BOX_VERTICAL)?;
    write!(out, "    {}  ", label)?;

    match window {
        None => {
            let line = "-";
            write!(out, "{}", theme::dim_style(line))?;
            let used = 4 + label.len() + 2 + line.len();
            let pad = inner_width.saturating_sub(used + 1);
            write!(out, "{}", " ".repeat(pad))?;
        }
        Some(w) => {
            let rem = remaining_percent(w.used_percent);
            let bar = theme::mini_progress_bar(rem);
            let _percent = format!(" {}%", rem);

            let reset_info = match w.resets_at {
                Some(r) if now < r => {
                    let dt = Local.timestamp_opt(r, 0).single();
                    match dt {
                        Some(dt) => format!("  重置 {}", dt.format("%H:%M")),
                        None => String::new(),
                    }
                }
                Some(_) => "  已重置".to_string(),
                None => String::new(),
            };

            // 告警标记
            let alert = if rem == 0 {
                format!("        {} 已耗尽", icons::CROSS)
            } else if rem < threshold {
                format!("        {} 额度不足", icons::WARN)
            } else {
                String::new()
            };

            write!(out, "{}", bar)?;
            write!(out, "{}", theme::colored_percent(rem))?;
            write!(out, "{}", theme::dim_style(&reset_info))?;
            if !alert.is_empty() {
                if rem == 0 {
                    write!(out, "{}", theme::error_style(&alert))?;
                } else {
                    write!(out, "{}", theme::warn_style(&alert))?;
                }
            }

            // 简化的填充（不精确计算 ANSI 转义序列宽度）
            write!(out, "  ")?;
        }
    }

    writeln!(out, "{}\r", icons::BOX_VERTICAL)?;
    Ok(())
}

/// 邮箱简短显示（UTF-8 安全截断）
fn short_email(email: &str) -> &str {
    if email.chars().count() <= 20 {
        email
    } else {
        match email.char_indices().nth(20) {
            Some((idx, _)) => &email[..idx],
            None => email,
        }
    }
}

/// 字符串截断（UTF-8 安全）
fn truncate_str(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else if max <= 1 {
        ".".to_string()
    } else {
        let end = s.char_indices().nth(max - 1).map(|(i, _)| i).unwrap_or(s.len());
        format!("{}.", &s[..end])
    }
}

/// RAII 守卫：退出时恢复终端
struct RawModeGuard;
impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();
        let mut out = io::stdout();
        let _ = out.execute(terminal::Clear(ClearType::All));
        let _ = out.execute(cursor::MoveTo(0, 0));
    }
}
