use crate::core::models::*;
use crate::core::{registry, sessions};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tiny_http::{Header, Response, Server};

/// Web 仪表盘状态
struct WebDashboardState {
    codex_home: PathBuf,
    interval: u64,
    threshold: u64,
    auto_switch: bool,
}

/// 启动 Web 仪表盘
pub fn run_web_dashboard(
    codex_home: &Path,
    interval: u64,
    threshold: u64,
    auto_switch: bool,
    port: u16,
) -> anyhow::Result<()> {
    let addr = format!("127.0.0.1:{}", port);
    let server = Server::http(&addr)
        .map_err(|e| anyhow::anyhow!("无法启动 HTTP 服务器: {}", e))?;

    println!("🌐 Web 仪表盘已启动！");
    println!("   访问: http://localhost:{}", port);
    println!("   按 Ctrl+C 退出");
    println!();

    let state = Arc::new(Mutex::new(WebDashboardState {
        codex_home: codex_home.to_path_buf(),
        interval,
        threshold,
        auto_switch,
    }));

    // 启动后台线程定时刷新额度数据
    let bg_state = Arc::clone(&state);
    thread::spawn(move || {
        loop {
            let st = bg_state.lock().unwrap_or_else(|e| e.into_inner());
            let codex_home = st.codex_home.clone();
            let _threshold = st.threshold;
            let _auto_switch = st.auto_switch;
            let interval = st.interval;
            drop(st);

            // 刷新额度数据
            if let Ok(mut reg) = registry::load_registry(&codex_home) {
                let _ = registry::sync_active_account_from_auth(&codex_home, &mut reg);
                if let Ok(Some(snapshot)) = sessions::scan_latest_usage(&codex_home) {
                    if let Some(active) = reg.active_email.clone() {
                        registry::update_usage(&mut reg, &active, snapshot);
                        let _ = registry::save_registry(&codex_home, &mut reg);
                    }
                }
            }

            thread::sleep(Duration::from_secs(interval));
        }
    });

    // HTTP 请求处理循环
    for request in server.incoming_requests() {
        let url = request.url().to_string();
        let st = state.lock().unwrap_or_else(|e| e.into_inner());
        let codex_home = st.codex_home.clone();
        let interval = st.interval;
        let threshold = st.threshold;
        let auto_switch = st.auto_switch;
        drop(st);

        match url.as_str() {
            "/" => {
                // 返回 HTML 页面
                let html = build_dashboard_html(interval, threshold, auto_switch);
                let header = Header::from_bytes(
                    &b"Content-Type"[..],
                    &b"text/html; charset=utf-8"[..],
                )
                .unwrap();
                let response = Response::from_string(html).with_header(header);
                let _ = request.respond(response);
            }
            "/api/status" => {
                // 返回 JSON 数据
                let json = build_status_json(&codex_home);
                let header = Header::from_bytes(
                    &b"Content-Type"[..],
                    &b"application/json; charset=utf-8"[..],
                )
                .unwrap();
                let response = Response::from_string(json)
                    .with_header(header);
                let _ = request.respond(response);
            }
            _ => {
                let response = Response::from_string("Not Found")
                    .with_status_code(404);
                let _ = request.respond(response);
            }
        }
    }

    Ok(())
}

/// 构建 /api/status 的 JSON 响应
fn build_status_json(codex_home: &Path) -> String {
    let reg = match registry::load_registry(codex_home) {
        Ok(r) => r,
        Err(_) => return r#"{"error":"无法加载注册表"}"#.to_string(),
    };

    let now = registry::now_timestamp();

    let mut accounts = Vec::new();
    for rec in &reg.accounts {
        let is_active = reg.active_email.as_deref() == Some(&rec.email);
        let plan = resolve_plan(rec)
            .map(|p| p.to_string())
            .unwrap_or_else(|| "-".to_string());

        let rate_5h = resolve_rate_window(&rec.last_usage, 300, true);
        let rate_week = resolve_rate_window(&rec.last_usage, 10080, false);

        let fmt_window = |w: Option<&RateLimitWindow>| -> serde_json::Value {
            match w {
                None => serde_json::json!(null),
                Some(w) => {
                    let remaining = remaining_percent(w.used_percent);
                    let reset_info = match w.resets_at {
                        Some(r) if now < r => {
                            let secs = r - now;
                            let mins = secs / 60;
                            let hours = mins / 60;
                            if hours > 0 {
                                format!("{}h{}m", hours, mins % 60)
                            } else {
                                format!("{}m", mins)
                            }
                        }
                        Some(_) => "已重置".to_string(),
                        None => "-".to_string(),
                    };
                    serde_json::json!({
                        "used_percent": w.used_percent,
                        "remaining_percent": remaining,
                        "resets_in": reset_info,
                        "resets_at": w.resets_at,
                    })
                }
            }
        };

        accounts.push(serde_json::json!({
            "email": rec.email,
            "alias": rec.alias,
            "plan": plan,
            "is_active": is_active,
            "rate_5h": fmt_window(rate_5h),
            "rate_week": fmt_window(rate_week),
            "last_used_at": rec.last_used_at,
            "last_usage_at": rec.last_usage_at,
        }));
    }

    let result = serde_json::json!({
        "timestamp": now,
        "accounts": accounts,
    });

    serde_json::to_string(&result).unwrap_or_else(|_| r#"{"error":"序列化失败"}"#.to_string())
}

/// 构建仪表盘 HTML 页面
fn build_dashboard_html(interval: u64, threshold: u64, auto_switch: bool) -> String {
    format!(
        r##"<!DOCTYPE html>
<html lang="zh-CN">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>CX-Switch 额度监控仪表盘</title>
<link rel="preconnect" href="https://fonts.googleapis.com">
<link href="https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@400;600;700&family=Inter:wght@400;500;600;700&display=swap" rel="stylesheet">
<style>
:root {{
  --bg-primary: #0a0e17;
  --bg-secondary: #111827;
  --bg-card: #1a2332;
  --bg-card-hover: #1f2b3d;
  --border-color: #2a3a50;
  --border-active: #3b82f6;
  --text-primary: #e2e8f0;
  --text-secondary: #94a3b8;
  --text-muted: #64748b;
  --accent-blue: #3b82f6;
  --accent-green: #10b981;
  --accent-yellow: #f59e0b;
  --accent-red: #ef4444;
  --accent-purple: #8b5cf6;
  --glow-blue: rgba(59, 130, 246, 0.15);
  --glow-green: rgba(16, 185, 129, 0.15);
}}

* {{ margin: 0; padding: 0; box-sizing: border-box; }}

body {{
  font-family: 'Inter', -apple-system, sans-serif;
  background: var(--bg-primary);
  color: var(--text-primary);
  min-height: 100vh;
  overflow-x: hidden;
}}

/* 背景动画 */
body::before {{
  content: '';
  position: fixed;
  top: 0; left: 0;
  width: 100%; height: 100%;
  background:
    radial-gradient(ellipse at 20% 50%, rgba(59, 130, 246, 0.06) 0%, transparent 50%),
    radial-gradient(ellipse at 80% 20%, rgba(139, 92, 246, 0.04) 0%, transparent 50%),
    radial-gradient(ellipse at 50% 80%, rgba(16, 185, 129, 0.04) 0%, transparent 50%);
  pointer-events: none;
  z-index: 0;
}}

.container {{
  max-width: 960px;
  margin: 0 auto;
  padding: 2rem 1.5rem;
  position: relative;
  z-index: 1;
}}

/* 顶部 Header */
.header {{
  text-align: center;
  margin-bottom: 2.5rem;
}}

.header h1 {{
  font-size: 1.75rem;
  font-weight: 700;
  background: linear-gradient(135deg, var(--accent-blue), var(--accent-purple));
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  background-clip: text;
  margin-bottom: 0.5rem;
}}

.header .subtitle {{
  font-size: 0.85rem;
  color: var(--text-muted);
  font-family: 'JetBrains Mono', monospace;
}}

.status-bar {{
  display: flex;
  justify-content: center;
  gap: 2rem;
  margin-top: 1rem;
  flex-wrap: wrap;
}}

.status-item {{
  display: flex;
  align-items: center;
  gap: 0.4rem;
  font-size: 0.8rem;
  color: var(--text-secondary);
}}

.status-dot {{
  width: 6px; height: 6px;
  border-radius: 50%;
  background: var(--accent-green);
  animation: pulse 2s infinite;
}}

@keyframes pulse {{
  0%, 100% {{ opacity: 1; }}
  50% {{ opacity: 0.4; }}
}}

/* 账号卡片 */
.accounts {{
  display: flex;
  flex-direction: column;
  gap: 1rem;
}}

.account-card {{
  background: var(--bg-card);
  border: 1px solid var(--border-color);
  border-radius: 12px;
  padding: 1.25rem 1.5rem;
  transition: all 0.3s ease;
}}

.account-card:hover {{
  background: var(--bg-card-hover);
  border-color: rgba(59, 130, 246, 0.3);
  box-shadow: 0 4px 20px rgba(0, 0, 0, 0.2);
  transform: translateY(-1px);
}}

.account-card.active {{
  border-color: var(--border-active);
  box-shadow: 0 0 0 1px var(--border-active), 0 4px 20px var(--glow-blue);
}}

.card-header {{
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 1rem;
}}

.email-section {{
  display: flex;
  align-items: center;
  gap: 0.6rem;
}}

.active-icon {{
  font-size: 1.1rem;
}}

.email {{
  font-family: 'JetBrains Mono', monospace;
  font-size: 0.9rem;
  font-weight: 600;
  color: var(--text-primary);
}}

.badges {{
  display: flex;
  gap: 0.5rem;
}}

.badge {{
  padding: 0.2rem 0.6rem;
  border-radius: 6px;
  font-size: 0.7rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}}

.badge-plan {{
  background: rgba(139, 92, 246, 0.15);
  color: var(--accent-purple);
  border: 1px solid rgba(139, 92, 246, 0.3);
}}

.badge-active {{
  background: rgba(16, 185, 129, 0.15);
  color: var(--accent-green);
  border: 1px solid rgba(16, 185, 129, 0.3);
}}

/* 额度行 */
.rate-rows {{
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}}

.rate-row {{
  display: flex;
  align-items: center;
  gap: 0.75rem;
}}

.rate-label {{
  font-family: 'JetBrains Mono', monospace;
  font-size: 0.75rem;
  font-weight: 600;
  color: var(--text-muted);
  width: 50px;
  flex-shrink: 0;
}}

.progress-wrap {{
  flex: 1;
  display: flex;
  align-items: center;
  gap: 0.75rem;
}}

.progress-bar {{
  flex: 1;
  height: 8px;
  background: rgba(255, 255, 255, 0.06);
  border-radius: 4px;
  overflow: hidden;
  position: relative;
}}

.progress-fill {{
  height: 100%;
  border-radius: 4px;
  transition: width 0.8s cubic-bezier(0.4, 0, 0.2, 1);
  position: relative;
}}

.progress-fill::after {{
  content: '';
  position: absolute;
  top: 0; left: 0; right: 0; bottom: 0;
  background: linear-gradient(90deg, transparent, rgba(255,255,255,0.15), transparent);
  animation: shimmer 2s infinite;
}}

@keyframes shimmer {{
  0% {{ transform: translateX(-100%); }}
  100% {{ transform: translateX(100%); }}
}}

.progress-fill.green {{ background: linear-gradient(90deg, #059669, #10b981); }}
.progress-fill.yellow {{ background: linear-gradient(90deg, #d97706, #f59e0b); }}
.progress-fill.red {{ background: linear-gradient(90deg, #dc2626, #ef4444); }}

.rate-percent {{
  font-family: 'JetBrains Mono', monospace;
  font-size: 0.85rem;
  font-weight: 700;
  width: 48px;
  text-align: right;
  flex-shrink: 0;
}}

.rate-reset {{
  font-size: 0.75rem;
  color: var(--text-muted);
  width: 80px;
  text-align: right;
  flex-shrink: 0;
  font-family: 'JetBrains Mono', monospace;
}}

.rate-empty {{
  font-size: 0.8rem;
  color: var(--text-muted);
  font-style: italic;
}}

/* 底部信息 */
.footer {{
  text-align: center;
  margin-top: 2rem;
  padding-top: 1.5rem;
  border-top: 1px solid var(--border-color);
}}

.footer p {{
  font-size: 0.75rem;
  color: var(--text-muted);
}}

.last-update {{
  font-family: 'JetBrains Mono', monospace;
}}

/* 无数据提示 */
.no-data {{
  text-align: center;
  padding: 3rem;
  color: var(--text-muted);
}}

.no-data .icon {{
  font-size: 3rem;
  margin-bottom: 1rem;
}}

/* 响应式 */
@media (max-width: 640px) {{
  .container {{ padding: 1rem; }}
  .header h1 {{ font-size: 1.4rem; }}
  .status-bar {{ gap: 1rem; }}
  .rate-reset {{ display: none; }}
  .card-header {{ flex-direction: column; align-items: flex-start; gap: 0.5rem; }}
}}
</style>
</head>
<body>
<div class="container">
  <div class="header">
    <h1>🔄 CX-Switch 额度监控</h1>
    <p class="subtitle">Codex 多账号管理仪表盘</p>
    <div class="status-bar">
      <span class="status-item"><span class="status-dot"></span>运行中</span>
      <span class="status-item">⏱ 每 {interval}s 刷新</span>
      <span class="status-item">🎯 阈值 {threshold}%</span>
      <span class="status-item">🔄 自动切换: {auto_switch_text}</span>
    </div>
  </div>

  <div id="accounts" class="accounts">
    <div class="no-data">
      <div class="icon">⏳</div>
      <p>正在加载...</p>
    </div>
  </div>

  <div class="footer">
    <p>上次刷新: <span class="last-update" id="lastUpdate">-</span></p>
  </div>
</div>

<script>
const INTERVAL = {interval} * 1000;
const THRESHOLD = {threshold};

function escapeHtml(str) {{
  const div = document.createElement('div');
  div.textContent = str;
  return div.innerHTML;
}}

function getColor(remaining) {{
  if (remaining <= 0) return 'red';
  if (remaining < THRESHOLD) return 'yellow';
  return 'green';
}}

function getTextColor(remaining) {{
  if (remaining <= 0) return 'var(--accent-red)';
  if (remaining < THRESHOLD) return 'var(--accent-yellow)';
  return 'var(--accent-green)';
}}

function renderRateRow(label, data) {{
  if (!data) {{
    return `
      <div class="rate-row">
        <span class="rate-label">${{label}}</span>
        <span class="rate-empty">暂无数据</span>
      </div>`;
  }}
  const rem = data.remaining_percent;
  const color = getColor(rem);
  const textColor = getTextColor(rem);
  return `
    <div class="rate-row">
      <span class="rate-label">${{label}}</span>
      <div class="progress-wrap">
        <div class="progress-bar">
          <div class="progress-fill ${{color}}" style="width: ${{rem}}%"></div>
        </div>
        <span class="rate-percent" style="color: ${{textColor}}">${{rem}}%</span>
        <span class="rate-reset">${{data.resets_in || '-'}}</span>
      </div>
    </div>`;
}}

function renderAccount(acc) {{
  const activeClass = acc.is_active ? 'active' : '';
  const activeIcon = acc.is_active ? '✅' : '⬜';
  const activeBadge = acc.is_active ? '<span class="badge badge-active">ACTIVE</span>' : '';

  return `
    <div class="account-card ${{activeClass}}">
      <div class="card-header">
        <div class="email-section">
          <span class="active-icon">${{activeIcon}}</span>
          <span class="email">${{escapeHtml(acc.email)}}</span>
        </div>
        <div class="badges">
          <span class="badge badge-plan">${{escapeHtml(acc.plan)}}</span>
          ${{activeBadge}}
        </div>
      </div>
      <div class="rate-rows">
        ${{renderRateRow('5H', acc.rate_5h)}}
        ${{renderRateRow('WEEK', acc.rate_week)}}
      </div>
    </div>`;
}}

async function refresh() {{
  try {{
    const res = await fetch('/api/status');
    const data = await res.json();

    if (data.error) {{
      document.getElementById('accounts').innerHTML =
        `<div class="no-data"><div class="icon">❌</div><p>${{escapeHtml(data.error)}}</p></div>`;
      return;
    }}

    if (!data.accounts || data.accounts.length === 0) {{
      document.getElementById('accounts').innerHTML =
        `<div class="no-data"><div class="icon">📭</div><p>暂无已管理的账号<br>使用 cx-switch login 添加账号</p></div>`;
      return;
    }}

    const html = data.accounts.map(renderAccount).join('');
    document.getElementById('accounts').innerHTML = html;

    const now = new Date();
    document.getElementById('lastUpdate').textContent =
      now.toLocaleTimeString('zh-CN', {{ hour12: false }});
  }} catch (e) {{
    console.error('刷新失败:', e);
  }}
}}

// 初始加载 + 定时刷新
refresh();
setInterval(refresh, INTERVAL);
</script>
</body>
</html>"##,
        interval = interval,
        threshold = threshold,
        auto_switch_text = if auto_switch { "开启" } else { "关闭" },
    )
}
