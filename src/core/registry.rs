use anyhow::{Context, Result};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use super::auth;
use super::models::*;

/// 解析 Codex 主目录路径
pub fn resolve_codex_home() -> Result<PathBuf> {
    // 优先使用 CODEX_HOME 环境变量
    if let Ok(val) = std::env::var("CODEX_HOME") {
        if !val.is_empty() {
            return Ok(PathBuf::from(val));
        }
    }

    // 使用 dirs crate 获取 HOME 目录
    if let Some(home) = dirs::home_dir() {
        return Ok(home.join(".codex"));
    }

    anyhow::bail!("无法确定 Codex 主目录：请设置 CODEX_HOME 环境变量或确保 HOME 目录可用")
}

/// 获取 accounts 目录路径
fn accounts_dir(codex_home: &Path) -> PathBuf {
    codex_home.join("accounts")
}

/// 确保 accounts 目录存在
pub fn ensure_accounts_dir(codex_home: &Path) -> Result<()> {
    let dir = accounts_dir(codex_home);
    fs::create_dir_all(&dir).with_context(|| format!("无法创建目录: {}", dir.display()))
}

/// 获取注册表文件路径
pub fn registry_path(codex_home: &Path) -> PathBuf {
    codex_home.join("accounts").join("registry.json")
}

/// 将邮箱编码为文件名键（base64url 无填充）
fn email_file_key(email: &str) -> String {
    URL_SAFE_NO_PAD.encode(email.as_bytes())
}

/// 获取账号认证文件路径
pub fn account_auth_path(codex_home: &Path, email: &str) -> PathBuf {
    let key = email_file_key(email);
    let filename = format!("{}.auth.json", key);
    codex_home.join("accounts").join(filename)
}

/// 获取当前活跃认证文件路径
pub fn active_auth_path(codex_home: &Path) -> PathBuf {
    codex_home.join("auth.json")
}

/// 复制文件（认证文件设置 0600 权限）
pub fn copy_file(src: &Path, dest: &Path) -> Result<()> {
    fs::copy(src, dest).with_context(|| {
        format!(
            "复制文件失败: {} -> {}",
            src.display(),
            dest.display()
        )
    })?;
    set_file_permissions_private(dest);
    Ok(())
}

/// 设置文件为仅拥有者可读写（Unix: 0600）
fn set_file_permissions_private(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
    }
    let _ = path; // suppress unused warning on non-unix
}

/// 获取当前 Unix 时间戳
pub fn now_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

/// 获取当前毫秒时间戳
fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

const MAX_BACKUPS: usize = 5;

// ─── 注册表加载/保存 ───

/// 加载注册表
pub fn load_registry(codex_home: &Path) -> Result<Registry> {
    let path = registry_path(codex_home);
    if !path.exists() {
        return Ok(Registry::new());
    }

    let data = fs::read_to_string(&path)
        .with_context(|| format!("读取注册表失败: {}", path.display()))?;

    let root: serde_json::Value =
        serde_json::from_str(&data).with_context(|| "注册表 JSON 解析失败")?;

    let obj = match root.as_object() {
        Some(o) => o,
        None => return Ok(Registry::new()),
    };

    load_registry_v2(obj)
}

/// 解析 v2 格式注册表
fn load_registry_v2(obj: &serde_json::Map<String, serde_json::Value>) -> Result<Registry> {
    let mut reg = Registry::new();

    // 读取 active_email
    if let Some(v) = obj.get("active_email") {
        if let Some(s) = v.as_str() {
            reg.active_email = Some(s.to_lowercase());
        }
    }

    // 读取 accounts 数组
    if let Some(v) = obj.get("accounts") {
        if let Some(arr) = v.as_array() {
            for item in arr {
                if let Some(item_obj) = item.as_object() {
                    if let Some(rec) = parse_account_record(item_obj) {
                        upsert_account(&mut reg, rec);
                    }
                }
            }
        }
    }

    Ok(reg)
}

/// 从 JSON 对象解析账号记录
fn parse_account_record(
    obj: &serde_json::Map<String, serde_json::Value>,
) -> Option<AccountRecord> {
    let email = obj.get("email")?.as_str()?.to_lowercase();
    let alias = obj
        .get("alias")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let mut rec = AccountRecord {
        email,
        alias,
        plan: None,
        auth_mode: None,
        created_at: obj
            .get("created_at")
            .and_then(|v| v.as_i64())
            .unwrap_or_else(now_timestamp),
        last_used_at: None,
        last_usage: None,
        last_usage_at: None,
    };

    if let Some(p) = obj.get("plan").and_then(|v| v.as_str()) {
        rec.plan = Some(PlanType::from_str_loose(p));
    }
    if let Some(m) = obj.get("auth_mode").and_then(|v| v.as_str()) {
        rec.auth_mode = parse_auth_mode(m);
    }
    rec.last_used_at = obj.get("last_used_at").and_then(|v| v.as_i64());
    rec.last_usage_at = obj.get("last_usage_at").and_then(|v| v.as_i64());

    if let Some(u) = obj.get("last_usage") {
        rec.last_usage = parse_usage(u);
    }

    Some(rec)
}

fn parse_auth_mode(s: &str) -> Option<AuthMode> {
    match s {
        "chatgpt" => Some(AuthMode::Chatgpt),
        "apikey" => Some(AuthMode::Apikey),
        _ => None,
    }
}

fn parse_usage(v: &serde_json::Value) -> Option<RateLimitSnapshot> {
    let obj = v.as_object()?;
    let mut snap = RateLimitSnapshot {
        primary: None,
        secondary: None,
        credits: None,
        plan_type: None,
    };

    if let Some(p) = obj.get("plan_type").and_then(|v| v.as_str()) {
        snap.plan_type = Some(PlanType::from_str_loose(p));
    }
    if let Some(p) = obj.get("primary") {
        snap.primary = parse_window(p);
    }
    if let Some(s) = obj.get("secondary") {
        snap.secondary = parse_window(s);
    }
    if let Some(c) = obj.get("credits") {
        snap.credits = parse_credits(c);
    }

    Some(snap)
}

fn parse_window(v: &serde_json::Value) -> Option<RateLimitWindow> {
    let obj = v.as_object()?;
    let used_percent = match obj.get("used_percent")? {
        serde_json::Value::Number(n) => n.as_f64().unwrap_or(0.0),
        _ => 0.0,
    };
    let window_minutes = obj.get("window_minutes").and_then(|v| v.as_i64());
    let resets_at = obj.get("resets_at").and_then(|v| v.as_i64());

    Some(RateLimitWindow {
        used_percent,
        window_minutes,
        resets_at,
    })
}

fn parse_credits(v: &serde_json::Value) -> Option<CreditsSnapshot> {
    let obj = v.as_object()?;
    Some(CreditsSnapshot {
        has_credits: obj.get("has_credits").and_then(|v| v.as_bool()).unwrap_or(false),
        unlimited: obj.get("unlimited").and_then(|v| v.as_bool()).unwrap_or(false),
        balance: obj.get("balance").and_then(|v| v.as_str()).map(|s| s.to_string()),
    })
}

/// 保存注册表
pub fn save_registry(codex_home: &Path, reg: &mut Registry) -> Result<()> {
    reg.version = 2;
    ensure_accounts_dir(codex_home)?;

    let path = registry_path(codex_home);
    let data = serde_json::to_string_pretty(reg).with_context(|| "注册表序列化失败")?;

    // 内容相同则跳过
    if file_equals_bytes(&path, data.as_bytes()) {
        return Ok(());
    }

    // 备份旧注册表
    backup_registry_if_changed(codex_home, &path, data.as_bytes())?;

    let mut file = fs::File::create(&path)
        .with_context(|| format!("创建注册表文件失败: {}", path.display()))?;
    file.write_all(data.as_bytes())?;
    set_file_permissions_private(&path);

    Ok(())
}

// ─── 账号操作 ───

/// 插入或更新账号记录（邮箱匹配则更新）
pub fn upsert_account(reg: &mut Registry, record: AccountRecord) {
    for existing in reg.accounts.iter_mut() {
        if existing.email == record.email {
            // 合并：保留更新鲜的记录
            if record_freshness(&record) > record_freshness(existing) {
                *existing = record;
            }
            return;
        }
    }
    reg.accounts.push(record);
}

/// 计算记录新鲜度（最大的时间戳）
fn record_freshness(rec: &AccountRecord) -> i64 {
    let mut best = rec.created_at;
    if let Some(t) = rec.last_used_at {
        if t > best {
            best = t;
        }
    }
    if let Some(t) = rec.last_usage_at {
        if t > best {
            best = t;
        }
    }
    best
}

/// 设置活跃账号
pub fn set_active_account(reg: &mut Registry, email: &str) {
    if reg.active_email.as_deref() == Some(email) {
        return;
    }
    reg.active_email = Some(email.to_string());
    let now = now_timestamp();
    for rec in reg.accounts.iter_mut() {
        if rec.email == email {
            rec.last_used_at = Some(now);
            break;
        }
    }
}

/// 从 AuthInfo 创建账号记录
pub fn account_from_auth(alias: &str, info: &AuthInfo) -> Result<AccountRecord> {
    let email = info
        .email
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("认证文件中缺少邮箱信息"))?;

    Ok(AccountRecord {
        email: email.clone(),
        alias: alias.to_string(),
        plan: info.plan.clone(),
        auth_mode: Some(info.auth_mode.clone()),
        created_at: now_timestamp(),
        last_used_at: None,
        last_usage: None,
        last_usage_at: None,
    })
}

/// 更新账号的额度使用信息
pub fn update_usage(reg: &mut Registry, email: &str, snapshot: RateLimitSnapshot) {
    let now = now_timestamp();
    for rec in reg.accounts.iter_mut() {
        if rec.email == email {
            rec.last_usage = Some(snapshot);
            rec.last_usage_at = Some(now);
            break;
        }
    }
}

/// 同步当前 auth.json 到注册表（检测外部登录变更）
pub fn sync_active_account_from_auth(codex_home: &Path, reg: &mut Registry) -> Result<bool> {
    if reg.accounts.is_empty() {
        return auto_import_active_auth(codex_home, reg);
    }

    let auth_path = active_auth_path(codex_home);
    if !auth_path.exists() {
        return Ok(false);
    }

    let info = auth::parse_auth_info(auth_path.to_str().unwrap_or_default())?;
    let email = match &info.email {
        Some(e) => e.clone(),
        None => {
            eprintln!("auth.json 缺少邮箱信息，跳过同步");
            return Ok(false);
        }
    };

    let matched_index = reg.accounts.iter().position(|r| r.email == email);

    if matched_index.is_none() {
        // 新账号：自动导入
        let dest = account_auth_path(codex_home, &email);
        ensure_accounts_dir(codex_home)?;
        copy_file(&auth_path, &dest)?;

        let record = account_from_auth("", &info)?;
        upsert_account(reg, record);
        set_active_account(reg, &email);
        return Ok(true);
    }

    let idx = matched_index.unwrap();
    let mut changed = false;

    if reg.active_email.as_deref() != Some(&email) {
        changed = true;
    }

    // 更新 plan 和 auth_mode
    if info.plan.is_some() {
        reg.accounts[idx].plan = info.plan;
    }
    reg.accounts[idx].auth_mode = Some(info.auth_mode);

    // 同步认证文件
    let dest = account_auth_path(codex_home, &email);
    let auth_bytes = fs::read(&auth_path).unwrap_or_default();
    if !file_equals_bytes(&dest, &auth_bytes) {
        copy_file(&auth_path, &dest)?;
    }

    set_active_account(reg, &email);
    Ok(changed)
}

/// 自动导入当前 auth.json（注册表为空时）
fn auto_import_active_auth(codex_home: &Path, reg: &mut Registry) -> Result<bool> {
    if !reg.accounts.is_empty() {
        return Ok(false);
    }

    let auth_path = active_auth_path(codex_home);
    if !auth_path.exists() {
        return Ok(false);
    }

    let info = auth::parse_auth_info(auth_path.to_str().unwrap_or_default())?;
    let email = match &info.email {
        Some(e) => e.clone(),
        None => {
            eprintln!("auth.json 缺少邮箱信息，无法导入");
            return Ok(false);
        }
    };

    let dest = account_auth_path(codex_home, &email);
    ensure_accounts_dir(codex_home)?;
    copy_file(&auth_path, &dest)?;

    let record = account_from_auth("", &info)?;
    upsert_account(reg, record);
    set_active_account(reg, &email);
    Ok(true)
}

/// 从认证文件刷新全部账号的 plan/auth_mode 信息
pub fn refresh_accounts_from_auth(codex_home: &Path, reg: &mut Registry) -> Result<()> {
    for rec in reg.accounts.iter_mut() {
        let path = account_auth_path(codex_home, &rec.email);
        if !path.exists() {
            continue;
        }
        let info = auth::parse_auth_info(path.to_str().unwrap_or_default())?;
        let email = match &info.email {
            Some(e) => e.clone(),
            None => continue,
        };
        if email != rec.email {
            continue;
        }
        rec.plan = info.plan;
        rec.auth_mode = Some(info.auth_mode);
    }
    Ok(())
}

// ─── 导入 ───

/// 智能导入（文件或目录）
pub fn import_auth_path(
    codex_home: &Path,
    reg: &mut Registry,
    auth_path: &str,
    explicit_alias: Option<&str>,
) -> Result<ImportSummary> {
    let path = Path::new(auth_path);
    if path.is_dir() {
        if explicit_alias.is_some() {
            eprintln!("warning: --alias 在导入目录时会被忽略");
        }
        return import_auth_directory(codex_home, reg, path);
    }

    import_auth_file(codex_home, reg, path, explicit_alias)?;
    Ok(ImportSummary {
        imported: 1,
        skipped: 0,
    })
}

/// 导入单个认证文件
fn import_auth_file(
    codex_home: &Path,
    reg: &mut Registry,
    auth_file: &Path,
    explicit_alias: Option<&str>,
) -> Result<()> {
    let info = auth::parse_auth_info(auth_file.to_str().unwrap_or_default())?;
    let email = info
        .email
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("认证文件中缺少邮箱信息"))?;
    let alias = explicit_alias.unwrap_or("");

    let dest = account_auth_path(codex_home, email);
    ensure_accounts_dir(codex_home)?;
    copy_file(auth_file, &dest)?;

    let record = account_from_auth(alias, &info)?;
    upsert_account(reg, record);
    Ok(())
}

/// 批量导入目录下的所有 .json 文件
fn import_auth_directory(
    codex_home: &Path,
    reg: &mut Registry,
    dir_path: &Path,
) -> Result<ImportSummary> {
    let mut names: Vec<String> = Vec::new();

    for entry in fs::read_dir(dir_path)? {
        let entry = entry?;
        let ft = entry.file_type()?;
        if !ft.is_file() && !ft.is_symlink() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if name.ends_with(".json") {
            names.push(name);
        }
    }
    names.sort();

    let mut summary = ImportSummary::default();
    for name in &names {
        let file_path = dir_path.join(name);
        match import_auth_file(codex_home, reg, &file_path, None) {
            Ok(()) => summary.imported += 1,
            Err(e) => {
                summary.skipped += 1;
                eprintln!("跳过导入 {}: {}", file_path.display(), e);
            }
        }
    }
    Ok(summary)
}

// ─── 删除 ───

/// 删除指定索引的账号
pub fn remove_accounts(
    codex_home: &Path,
    reg: &mut Registry,
    indices: &[usize],
) -> Result<()> {
    if indices.is_empty() || reg.accounts.is_empty() {
        return Ok(());
    }

    let mut removed = vec![false; reg.accounts.len()];
    for &idx in indices {
        if idx < removed.len() {
            removed[idx] = true;
        }
    }

    // 检查活跃账号是否被删除
    if let Some(ref active) = reg.active_email {
        for (i, rec) in reg.accounts.iter().enumerate() {
            if removed[i] && rec.email == *active {
                reg.active_email = None;
                break;
            }
        }
    }

    // 删除认证文件并过滤账号列表
    let mut new_accounts = Vec::new();
    for (i, rec) in reg.accounts.iter().enumerate() {
        if removed[i] {
            let path = account_auth_path(codex_home, &rec.email);
            let _ = fs::remove_file(&path);
        } else {
            new_accounts.push(rec.clone());
        }
    }
    reg.accounts = new_accounts;

    Ok(())
}

/// 选择额度最充裕的账号索引
pub fn select_best_account_index_by_usage(reg: &Registry) -> Option<usize> {
    if reg.accounts.is_empty() {
        return None;
    }

    let mut best_idx: Option<usize> = None;
    let mut best_score: i64 = -2;
    let mut best_seen: i64 = -1;

    for (i, rec) in reg.accounts.iter().enumerate() {
        let score = usage_score(&rec.last_usage);
        let seen = rec.last_usage_at.unwrap_or(-1);
        if score > best_score || (score == best_score && seen > best_seen) {
            best_score = score;
            best_seen = seen;
            best_idx = Some(i);
        }
    }

    best_idx
}

/// 计算账号额度分数
fn usage_score(usage: &Option<RateLimitSnapshot>) -> i64 {
    let rate_5h = resolve_rate_window(usage, 300, true);
    let rate_week = resolve_rate_window(usage, 10080, false);
    let rem_5h = rate_5h.map(|w| remaining_percent(w.used_percent));
    let rem_week = rate_week.map(|w| remaining_percent(w.used_percent));

    match (rem_5h, rem_week) {
        (Some(a), Some(b)) => a.min(b),
        (Some(a), None) => a,
        (None, Some(b)) => b,
        (None, None) => -1,
    }
}

// ─── 备份 ───

/// 备份 auth.json（仅在内容变化时）
pub fn backup_auth_if_changed(
    codex_home: &Path,
    current_auth_path: &Path,
    new_auth_path: &Path,
) -> Result<()> {
    let dir = accounts_dir(codex_home);
    fs::create_dir_all(&dir)?;

    if files_equal(current_auth_path, new_auth_path) {
        return Ok(());
    }
    if !current_auth_path.exists() {
        return Ok(());
    }

    let backup = make_backup_path(&dir, "auth.json")?;
    fs::copy(current_auth_path, &backup)?;
    prune_backups(&dir, "auth.json", MAX_BACKUPS)?;
    Ok(())
}

/// 备份 registry.json（仅在内容变化时）
fn backup_registry_if_changed(
    codex_home: &Path,
    current_path: &Path,
    new_bytes: &[u8],
) -> Result<()> {
    let dir = accounts_dir(codex_home);
    fs::create_dir_all(&dir)?;

    if file_equals_bytes(current_path, new_bytes) {
        return Ok(());
    }
    if !current_path.exists() {
        return Ok(());
    }

    let backup = make_backup_path(&dir, "registry.json")?;
    fs::copy(current_path, &backup)?;
    prune_backups(&dir, "registry.json", MAX_BACKUPS)?;
    Ok(())
}

/// 比较两个文件内容是否相同
fn files_equal(a: &Path, b: &Path) -> bool {
    let a_data = fs::read(a).ok();
    let b_data = fs::read(b).ok();
    match (a_data, b_data) {
        (Some(a), Some(b)) => a == b,
        _ => false,
    }
}

/// 比较文件内容与字节数据是否相同
fn file_equals_bytes(path: &Path, bytes: &[u8]) -> bool {
    match fs::read(path) {
        Ok(data) => data == bytes,
        Err(_) => false,
    }
}

/// 生成唯一的备份路径
fn make_backup_path(dir: &Path, base_name: &str) -> Result<PathBuf> {
    let ts = now_millis();
    let base = format!("{}.bak.{}", base_name, ts);

    for attempt in 0..100 {
        let name = if attempt == 0 {
            base.clone()
        } else {
            format!("{}.{}", base, attempt)
        };
        let path = dir.join(&name);
        if !path.exists() {
            return Ok(path);
        }
    }

    anyhow::bail!("无法生成唯一的备份文件名")
}

/// 清理旧备份，保留最新的 max 份
fn prune_backups(dir: &Path, base_name: &str, max: usize) -> Result<()> {
    let mut entries: Vec<(PathBuf, SystemTime)> = Vec::new();

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with(base_name) || !name.contains(".bak.") {
            continue;
        }
        if let Ok(meta) = entry.metadata() {
            if meta.is_file() {
                let mtime = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
                entries.push((entry.path(), mtime));
            }
        }
    }

    // 按修改时间降序排列
    entries.sort_by(|a, b| b.1.cmp(&a.1));

    // 删除超出限额的旧备份
    for entry in entries.iter().skip(max) {
        let _ = fs::remove_file(&entry.0);
    }

    Ok(())
}
