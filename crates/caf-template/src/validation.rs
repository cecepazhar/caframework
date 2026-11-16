use crate::config::AppConfigFile;
pub use crate::error::ValidationError;
use regex::Regex;
use std::collections::HashSet;
use std::path::Path;
use std::sync::LazyLock;

static RE_SLUG: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-z][a-z0-9-]{1,30}$").expect("valid regex"));
static RE_CODE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[A-Z]{2,6}$").expect("valid regex"));
static RE_IDENTIFIER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-z0-9_-]+(\.[a-z0-9_-]+)+$").expect("valid regex"));
static RE_ROLE_KEY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-z][a-z0-9_]{0,31}$").expect("valid regex"));
static RE_URL_SAFE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-z0-9_-]+$").expect("valid regex"));
static RE_PHONE_E164: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\+[1-9]\d{6,14}$").expect("valid regex"));
static RE_EMAIL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[^@\s]+@[^@\s]+\.[^@\s]+$").expect("valid regex"));

pub const ALLOWED_ACCENTS: &[&str] = &[
    "violet", "sky", "emerald", "amber", "rose", "cyan", "indigo", "teal", "orange", "pink",
];
pub const ALLOWED_LOCALES: &[&str] = &["id", "en"];
pub const ALLOWED_PLATFORMS: &[&str] = &["linux", "windows", "macos", "android"];
pub const ALLOWED_FORMATS: &[&str] = &["csv", "xlsx", "json"];
pub const RESERVED_MENU_KEYS: &[&str] = &["settings", "lock", "_dev", "api"];
pub const SYSTEM_RESOURCES: &[&str] = &[
    "users",
    "roles",
    "settings",
    "security",
    "backup",
    "data",
    "integrations",
    "scheduler",
    "ai",
    "pro",
    "audit",
    "notes",
];

pub fn safe_prefix(source: &str, mut pos: usize) -> &str {
    pos = pos.min(source.len());
    while pos > 0 && !source.is_char_boundary(pos) {
        pos -= 1;
    }
    &source[..pos]
}

pub fn find_location(source: &str, needle: &str) -> (usize, usize) {
    if let Some(pos) = source.find(needle) {
        let prefix = safe_prefix(source, pos);
        let line = prefix.matches('\n').count() + 1;
        let col = prefix.rfind('\n').map_or(pos + 1, |nl| pos - nl);
        (line, col)
    } else {
        (1, 1)
    }
}

pub fn validate_app_toml(
    source: &str,
    project_root: Option<&Path>,
) -> Result<AppConfigFile, Vec<ValidationError>> {
    let mut errors = Vec::new();

    // 1. Raw TOML check for modules.backup
    if let Ok(raw_value) = toml::from_str::<toml::Value>(source) {
        if let Some(modules) = raw_value.get("modules").and_then(|m| m.as_table())
            && modules.contains_key("backup")
        {
            let (line, col) = find_location(source, "backup");
            errors.push(
                ValidationError::new(
                    "modules.backup",
                    "backup is mandatory and cannot be listed under [modules]",
                )
                .with_location(line, col),
            );
        }
        if raw_value.get("brand").is_none() {
            errors.push(ValidationError::new("brand", "brand is mandatory").with_location(1, 1));
        }
    }

    // 2. Deserialize with deny_unknown_fields
    let config: Result<AppConfigFile, toml::de::Error> = toml::from_str(source);
    let config = match config {
        Ok(c) => c,
        Err(e) => {
            let mut err = ValidationError::new("toml", e.message());
            if let Some(span) = e.span() {
                let prefix = safe_prefix(source, span.start);
                let line = prefix.matches('\n').count() + 1;
                let col = prefix
                    .rfind('\n')
                    .map_or(prefix.len() + 1, |nl| prefix.len() - nl);
                err.line = Some(line);
                err.column = Some(col);
            }
            errors.push(err);
            return Err(errors);
        }
    };

    // Rule 2: [app]
    if !RE_SLUG.is_match(&config.app.slug) {
        let (l, c) = find_location(source, &format!("slug = \"{}\"", config.app.slug));
        errors.push(
            ValidationError::new(
                "app.slug",
                format!(
                    "slug '{}' does not match ^[a-z][a-z0-9-]{{1,30}}$",
                    config.app.slug
                ),
            )
            .with_location(l, c),
        );
    }
    if !RE_CODE.is_match(&config.app.code) {
        let (l, c) = find_location(source, &format!("code = \"{}\"", config.app.code));
        errors.push(
            ValidationError::new(
                "app.code",
                format!("code '{}' does not match ^[A-Z]{{2,6}}$", config.app.code),
            )
            .with_location(l, c),
        );
    }
    if !RE_IDENTIFIER.is_match(&config.app.identifier) {
        let (l, c) = find_location(
            source,
            &format!("identifier = \"{}\"", config.app.identifier),
        );
        errors.push(
            ValidationError::new(
                "app.identifier",
                format!(
                    "identifier '{}' is not valid reverse-DNS format",
                    config.app.identifier
                ),
            )
            .with_location(l, c),
        );
    }
    if !ALLOWED_ACCENTS.contains(&config.app.accent.as_str()) {
        let (l, c) = find_location(source, &format!("accent = \"{}\"", config.app.accent));
        errors.push(
            ValidationError::new(
                "app.accent",
                format!(
                    "accent '{}' is not one of: {}",
                    config.app.accent,
                    ALLOWED_ACCENTS.join(", ")
                ),
            )
            .with_location(l, c),
        );
    }
    for loc in &config.app.locales {
        if !ALLOWED_LOCALES.contains(&loc.as_str()) {
            let (l, c) = find_location(source, loc);
            errors.push(
                ValidationError::new(
                    "app.locales",
                    format!("locale '{}' not in allowed set: {{id, en}}", loc),
                )
                .with_location(l, c),
            );
        }
    }
    if !config.app.locales.contains(&config.app.default_locale) {
        let (l, c) = find_location(
            source,
            &format!("default_locale = \"{}\"", config.app.default_locale),
        );
        errors.push(
            ValidationError::new(
                "app.default_locale",
                format!(
                    "default_locale '{}' must be included in locales {:?}",
                    config.app.default_locale, config.app.locales
                ),
            )
            .with_location(l, c),
        );
    }
    for plat in &config.app.platforms {
        if !ALLOWED_PLATFORMS.contains(&plat.as_str()) {
            let (l, c) = find_location(source, plat);
            errors.push(
                ValidationError::new(
                    "app.platforms",
                    format!(
                        "platform '{}' not in allowed set: {}",
                        plat,
                        ALLOWED_PLATFORMS.join(", ")
                    ),
                )
                .with_location(l, c),
            );
        }
    }

    // Rule 3: [brand]
    if config.brand.developer.trim().is_empty() {
        let (l, c) = find_location(source, "developer =");
        errors.push(
            ValidationError::new("brand.developer", "developer must not be empty")
                .with_location(l, c),
        );
    }
    if !config.brand.website.starts_with("https://") {
        let (l, c) = find_location(source, &config.brand.website);
        errors.push(
            ValidationError::new(
                "brand.website",
                format!("website '{}' must use https://", config.brand.website),
            )
            .with_location(l, c),
        );
    }
    if let Some(sup) = &config.brand.support_url
        && !sup.is_empty()
        && !sup.starts_with("https://")
    {
        let (l, c) = find_location(source, sup);
        errors.push(
            ValidationError::new(
                "brand.support_url",
                format!("support_url '{}' must use https://", sup),
            )
            .with_location(l, c),
        );
    }
    if !RE_EMAIL.is_match(&config.brand.email) {
        let (l, c) = find_location(source, &config.brand.email);
        errors.push(
            ValidationError::new(
                "brand.email",
                format!("email '{}' is not valid", config.brand.email),
            )
            .with_location(l, c),
        );
    }
    if !RE_PHONE_E164.is_match(&config.brand.phone) {
        let (l, c) = find_location(source, &config.brand.phone);
        errors.push(
            ValidationError::new(
                "brand.phone",
                format!(
                    "phone '{}' must be in E.164 format (e.g. +6285220696117)",
                    config.brand.phone
                ),
            )
            .with_location(l, c),
        );
    }
    if let Some(root) = project_root {
        let logo_path = root.join(&config.brand.logo);
        if !logo_path.exists() {
            let (l, c) = find_location(source, &config.brand.logo);
            errors.push(
                ValidationError::new(
                    "brand.logo",
                    format!(
                        "logo file '{}' does not exist at {:?}",
                        config.brand.logo, logo_path
                    ),
                )
                .with_location(l, c),
            );
        }
    }

    // Known resources collector
    let mut known_resources: HashSet<String> =
        SYSTEM_RESOURCES.iter().map(|s| s.to_string()).collect();
    for m in &config.menus {
        known_resources.insert(m.key.clone());
    }
    for e in &config.entities {
        known_resources.insert(e.key.clone());
    }
    for i in &config.integrations {
        known_resources.insert(i.key.clone());
    }
    for j in &config.jobs {
        known_resources.insert(j.key.clone());
    }

    // Rule 4: [rbac]
    let mut role_keys = HashSet::new();
    let mut super_role_found = false;
    for role in &config.rbac.roles {
        if !RE_ROLE_KEY.is_match(&role.key) {
            let (l, c) = find_location(source, &format!("key = \"{}\"", role.key));
            errors.push(
                ValidationError::new(
                    "rbac.roles.key",
                    format!(
                        "role key '{}' must match ^[a-z][a-z0-9_]{{0,31}}$",
                        role.key
                    ),
                )
                .with_location(l, c),
            );
        }
        if !role_keys.insert(role.key.clone()) {
            let (l, c) = find_location(source, &format!("key = \"{}\"", role.key));
            errors.push(
                ValidationError::new(
                    "rbac.roles.key",
                    format!("duplicate role key '{}'", role.key),
                )
                .with_location(l, c),
            );
        }
        if role.key == config.rbac.super_role {
            super_role_found = true;
            if role.permissions != vec!["*"] {
                let (l, c) = find_location(source, &format!("key = \"{}\"", role.key));
                errors.push(
                    ValidationError::new(
                        "rbac.super_role",
                        format!(
                            "super role '{}' must hold exactly [\"*\"]",
                            config.rbac.super_role
                        ),
                    )
                    .with_location(l, c),
                );
            }
        }
        for perm in &role.permissions {
            if perm == "*" {
                continue;
            }
            if let Some((res, action)) = perm.split_once(':') {
                if !known_resources.contains(res) {
                    let (l, c) = find_location(source, perm);
                    errors.push(
                        ValidationError::new(
                            "rbac.roles.permissions",
                            format!(
                                "permission '{}' references unknown resource '{}'",
                                perm, res
                            ),
                        )
                        .with_location(l, c),
                    );
                }
                if action != "*"
                    && !matches!(
                        action,
                        "view"
                            | "create"
                            | "update"
                            | "delete"
                            | "export"
                            | "import"
                            | "manage"
                            | "run"
                    )
                {
                    let (l, c) = find_location(source, perm);
                    errors.push(
                        ValidationError::new(
                            "rbac.roles.permissions",
                            format!("permission '{}' has unrecognized action '{}'", perm, action),
                        )
                        .with_location(l, c),
                    );
                }
            } else {
                let (l, c) = find_location(source, perm);
                errors.push(
                    ValidationError::new(
                        "rbac.roles.permissions",
                        format!("permission '{}' must match <resource>:<action> or *", perm),
                    )
                    .with_location(l, c),
                );
            }
        }
    }
    if !super_role_found {
        let (l, c) = find_location(
            source,
            &format!("super_role = \"{}\"", config.rbac.super_role),
        );
        errors.push(
            ValidationError::new(
                "rbac.super_role",
                format!(
                    "super_role '{}' not found in [[rbac.roles]]",
                    config.rbac.super_role
                ),
            )
            .with_location(l, c),
        );
    }
    if let Some(prof) = &config.profiles
        && let Some(pins) = &prof.pin_required_for
    {
        for pin_role in pins {
            if !role_keys.contains(pin_role) {
                let (l, c) = find_location(source, pin_role);
                errors.push(
                    ValidationError::new(
                        "profiles.pin_required_for",
                        format!(
                            "pin_required_for role '{}' is not defined in rbac.roles",
                            pin_role
                        ),
                    )
                    .with_location(l, c),
                );
            }
        }
    }

    // Rule 5: Keys unique and URL-safe, Cron parsing, [pro.features]
    let mut item_keys = HashSet::new();
    for intg in &config.integrations {
        if !RE_URL_SAFE.is_match(&intg.key) {
            let (l, c) = find_location(source, &format!("key = \"{}\"", intg.key));
            errors.push(
                ValidationError::new(
                    "integrations.key",
                    format!("integration key '{}' is not URL-safe", intg.key),
                )
                .with_location(l, c),
            );
        }
        if !item_keys.insert(format!("intg:{}", intg.key)) {
            let (l, c) = find_location(source, &format!("key = \"{}\"", intg.key));
            errors.push(
                ValidationError::new(
                    "integrations.key",
                    format!("duplicate integration key '{}'", intg.key),
                )
                .with_location(l, c),
            );
        }
    }
    for job in &config.jobs {
        if !RE_URL_SAFE.is_match(&job.key) && !job.key.contains('.') {
            let (l, c) = find_location(source, &format!("key = \"{}\"", job.key));
            errors.push(
                ValidationError::new("jobs.key", format!("job key '{}' is not URL-safe", job.key))
                    .with_location(l, c),
            );
        }
        if !item_keys.insert(format!("job:{}", job.key)) {
            let (l, c) = find_location(source, &format!("key = \"{}\"", job.key));
            errors.push(
                ValidationError::new("jobs.key", format!("duplicate job key '{}'", job.key))
                    .with_location(l, c),
            );
        }
        if !is_valid_cron_or_preset(&job.schedule) {
            let (l, c) = find_location(source, &job.schedule);
            errors.push(
                ValidationError::new(
                    "jobs.schedule",
                    format!("invalid schedule expression '{}'", job.schedule),
                )
                .with_location(l, c),
            );
        }
    }
    for ent in &config.entities {
        if !RE_URL_SAFE.is_match(&ent.key) {
            let (l, c) = find_location(source, &format!("key = \"{}\"", ent.key));
            errors.push(
                ValidationError::new(
                    "entities.key",
                    format!("entity key '{}' is not URL-safe", ent.key),
                )
                .with_location(l, c),
            );
        }
        if !item_keys.insert(format!("ent:{}", ent.key)) {
            let (l, c) = find_location(source, &format!("key = \"{}\"", ent.key));
            errors.push(
                ValidationError::new(
                    "entities.key",
                    format!("duplicate entity key '{}'", ent.key),
                )
                .with_location(l, c),
            );
        }
        for fmt in &ent.formats {
            if !ALLOWED_FORMATS.contains(&fmt.as_str()) {
                let (l, c) = find_location(source, fmt);
                errors.push(
                    ValidationError::new(
                        "entities.formats",
                        format!("format '{}' not in {{csv, xlsx, json}}", fmt),
                    )
                    .with_location(l, c),
                );
            }
        }
    }
    if let Some(pro) = &config.pro {
        if let Some(feats) = &pro.features {
            for (feat, val) in feats {
                if val != "free" && val != "pro" {
                    let (l, c) = find_location(source, val);
                    errors.push(
                        ValidationError::new(
                            format!("pro.features.{}", feat),
                            format!("feature tier '{}' must be 'free' or 'pro'", val),
                        )
                        .with_location(l, c),
                    );
                }
            }
        }
        if let Some(currs) = &pro.currencies {
            if currs.is_empty() {
                let (l, c) = find_location(source, "currencies =");
                errors.push(
                    ValidationError::new("pro.currencies", "currencies list must not be empty")
                        .with_location(l, c),
                );
            }
            for cur in currs {
                if cur != "IDR" && cur != "USD" {
                    let (l, c) = find_location(source, cur);
                    errors.push(
                        ValidationError::new(
                            "pro.currencies",
                            format!("currency '{}' not in {{IDR, USD}}", cur),
                        )
                        .with_location(l, c),
                    );
                }
            }
        }
        if let Some(grace) = pro.offline_grace_days
            && !(1..=30).contains(&grace)
        {
            let (l, c) = find_location(source, &format!("offline_grace_days = {}", grace));
            errors.push(
                ValidationError::new(
                    "pro.offline_grace_days",
                    format!("offline_grace_days {} must be in range 1..=30", grace),
                )
                .with_location(l, c),
            );
        }
        if let Some(refresh) = pro.refresh_hours
            && !(1..=24).contains(&refresh)
        {
            let (l, c) = find_location(source, &format!("refresh_hours = {}", refresh));
            errors.push(
                ValidationError::new(
                    "pro.refresh_hours",
                    format!("refresh_hours {} must be in range 1..=24", refresh),
                )
                .with_location(l, c),
            );
        }
    }

    // Rule 6: Endpoints
    if let Some(ep) = &config.endpoints {
        if let Some(gcc) = &ep.gcc_base_url
            && !gcc.is_empty()
            && !gcc.starts_with("https://")
            && !is_loopback(gcc)
        {
            let (l, c) = find_location(source, gcc);
            errors.push(
                ValidationError::new(
                    "endpoints.gcc_base_url",
                    format!("endpoint '{}' must use https://", gcc),
                )
                .with_location(l, c),
            );
        }
        if let Some(upd) = &ep.updater_manifest_url
            && !upd.is_empty()
            && !upd.starts_with("https://")
            && !is_loopback(upd)
        {
            let (l, c) = find_location(source, upd);
            errors.push(
                ValidationError::new(
                    "endpoints.updater_manifest_url",
                    format!("endpoint '{}' must use https://", upd),
                )
                .with_location(l, c),
            );
        }
    }

    // Rule 7: Menus
    if config.menus.is_empty() {
        let (l, c) = find_location(source, "[app]");
        errors.push(
            ValidationError::new("menus", "at least one [[menus]] entry is required")
                .with_location(l, c),
        );
    }
    let mut menu_keys = HashSet::new();
    for m in &config.menus {
        if !RE_SLUG.is_match(&m.key) && !RE_ROLE_KEY.is_match(&m.key) {
            let (l, c) = find_location(source, &format!("key = \"{}\"", m.key));
            errors.push(
                ValidationError::new(
                    "menus.key",
                    format!("menu key '{}' must match ^[a-z][a-z0-9-]{{0,31}}$", m.key),
                )
                .with_location(l, c),
            );
        }
        if RESERVED_MENU_KEYS.contains(&m.key.as_str()) {
            let (l, c) = find_location(source, &format!("key = \"{}\"", m.key));
            errors.push(
                ValidationError::new("menus.key", format!("menu key '{}' is reserved", m.key))
                    .with_location(l, c),
            );
        }
        if !menu_keys.insert(m.key.clone()) {
            let (l, c) = find_location(source, &format!("key = \"{}\"", m.key));
            errors.push(
                ValidationError::new("menus.key", format!("duplicate menu key '{}'", m.key))
                    .with_location(l, c),
            );
        }
        if m.icon.len() > 2000 {
            let (l, c) = find_location(source, &format!("key = \"{}\"", m.key));
            errors.push(
                ValidationError::new(
                    "menus.icon",
                    format!("menu icon length {} exceeds 2000 characters", m.icon.len()),
                )
                .with_location(l, c),
            );
        }
    }

    if errors.is_empty() {
        Ok(config)
    } else {
        Err(errors)
    }
}

fn is_loopback(url: &str) -> bool {
    url.starts_with("http://localhost") || url.starts_with("http://127.0.0.1")
}

pub fn is_valid_cron_or_preset(schedule: &str) -> bool {
    let s = schedule.trim();
    if matches!(s, "hourly" | "daily" | "weekly" | "monthly") {
        return true;
    }
    if s.starts_with("every ")
        && s.ends_with(" minutes")
        && let Ok(n) = s["every ".len()..s.len() - " minutes".len()]
            .trim()
            .parse::<u32>()
    {
        return n >= 5;
    }
    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.len() != 5 {
        return false;
    }
    // 5 fields: minute (0-59), hour (0-23), day (1-31), month (1-12), dow (0-7)
    let ranges = [(0, 59), (0, 23), (1, 31), (1, 12), (0, 7)];
    for (part, (min, max)) in parts.iter().zip(ranges.iter()) {
        if !is_valid_cron_field(part, *min, *max) {
            return false;
        }
    }
    true
}

fn is_valid_cron_field(field: &str, min: u32, max: u32) -> bool {
    if field == "*" {
        return true;
    }
    for sub in field.split(',') {
        if let Some((base, step)) = sub.split_once('/') {
            if base != "*" {
                if let Ok(v) = base.parse::<u32>() {
                    if v < min || v > max {
                        return false;
                    }
                } else {
                    return false;
                }
            }
            if let Ok(s) = step.parse::<u32>() {
                if s == 0 {
                    return false;
                }
            } else {
                return false;
            }
        } else if let Some((start, end)) = sub.split_once('-') {
            if let (Ok(s), Ok(e)) = (start.parse::<u32>(), end.parse::<u32>()) {
                if s < min || e > max || s > e {
                    return false;
                }
            } else {
                return false;
            }
        } else if let Ok(val) = sub.parse::<u32>() {
            if val < min || val > max {
                return false;
            }
        } else {
            return false;
        }
    }
    true
}
