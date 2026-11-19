use caf_core::session::{self, Session};
use caf_core::vault;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

static TEST_LOCK: Mutex<()> = Mutex::new(());

struct TestEnv {
    dir: PathBuf,
    _guard: std::sync::MutexGuard<'static, ()>,
}

impl TestEnv {
    fn new(label: &str) -> Self {
        let guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        vault::lock();
        session::clear_current_session().ok();
        let dir = std::env::temp_dir().join(format!("caf_p16_{label}_{}", uuid::Uuid::new_v4()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create test dir");
        #[allow(clippy::disallowed_methods)]
        unsafe {
            std::env::set_var("CAF_DATA_DIR", &dir);
        }
        vault::setup_vault("test-p16-password-123").expect("setup vault");
        Self { dir, _guard: guard }
    }
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        vault::lock();
        session::clear_current_session().ok();
        let _ = fs::remove_dir_all(&self.dir);
        #[allow(clippy::disallowed_methods)]
        unsafe {
            std::env::remove_var("CAF_DATA_DIR");
        }
    }
}

#[test]
fn test_p16_1_zero_bottom_nav_and_all_menus() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("repo root");

    let frontend_src = repo_root.join("frontend/src");
    if !frontend_src.exists() {
        return;
    }

    // Verify BottomNav.svelte and AllMenusSheet.svelte files are gone
    let bottom_nav = frontend_src.join("lib/components/BottomNav.svelte");
    let all_menus = frontend_src.join("lib/components/AllMenusSheet.svelte");
    assert!(
        !bottom_nav.exists(),
        "BottomNav.svelte must be completely removed per A-04 / P16.1"
    );
    assert!(
        !all_menus.exists(),
        "AllMenusSheet.svelte must be completely removed per A-04 / P16.1"
    );

    // Recursively scan frontend/src to ensure zero mentions of BottomNav or AllMenusSheet
    fn scan_dir(dir: &Path) {
        for entry in fs::read_dir(dir).expect("read dir") {
            let entry = entry.expect("entry");
            let path = entry.path();
            if path.is_dir() {
                scan_dir(&path);
            } else if path.is_file() {
                let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
                if ext == "svelte" || ext == "ts" || ext == "js" {
                    let content = fs::read_to_string(&path).expect("read file");
                    assert!(
                        !content.contains("BottomNav"),
                        "File {:?} must not reference BottomNav",
                        path
                    );
                    assert!(
                        !content.contains("AllMenusSheet"),
                        "File {:?} must not reference AllMenusSheet",
                        path
                    );
                }
            }
        }
    }

    scan_dir(&frontend_src);
}

#[test]
fn test_p16_2_session_permissions_core_api_and_matrix() {
    let _env = TestEnv::new("p16_session_permissions");

    // 1. When no session is active, session_permissions returns empty vec
    session::clear_current_session().expect("clear session");
    let perms_empty = session::session_permissions().expect("session_permissions when logged out");
    assert!(
        perms_empty.is_empty(),
        "Logged out session must yield empty permissions"
    );

    // 2. Set viewer session
    session::set_current_session(Session::new("test-viewer", "viewer"))
        .expect("set viewer session");
    let perms_viewer = session::session_permissions().expect("session_permissions for viewer");
    assert!(
        !perms_viewer.is_empty(),
        "Viewer must have at least one permission"
    );
    assert!(
        perms_viewer.iter().any(|p| p == "notes:view"),
        "Viewer must hold notes:view permission"
    );
    assert!(
        !perms_viewer.iter().any(|p| p == "*"),
        "Viewer must NOT hold super role wildcard"
    );

    // 3. Set super_admin session
    session::set_current_session(Session::new("test-super", "super_admin"))
        .expect("set super_admin session");
    let perms_super = session::session_permissions().expect("session_permissions for super_admin");
    assert!(
        perms_super.iter().any(|p| p == "*"),
        "Super admin must hold '*' wildcard permission"
    );
}

#[test]
fn test_p16_3_no_tailwind_template_string_interpolation() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("repo root");

    let frontend_src = repo_root.join("frontend/src");
    if !frontend_src.exists() {
        return;
    }

    // Verify accent.ts exists and provides literal map
    let accent_file = frontend_src.join("lib/accent.ts");
    assert!(
        accent_file.exists(),
        "frontend/src/lib/accent.ts must exist"
    );
    let accent_content = fs::read_to_string(&accent_file).expect("read accent.ts");
    assert!(accent_content.contains("ACCENT_MAP"));
    assert!(accent_content.contains("getAppAccent"));

    // Check that no file builds dynamic Tailwind prefixes like bg-${ or text-${
    fn check_tailwind_templates(dir: &Path) {
        for entry in fs::read_dir(dir).expect("read dir") {
            let entry = entry.expect("entry");
            let path = entry.path();
            if path.is_dir() {
                check_tailwind_templates(&path);
            } else if path.is_file() {
                let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
                if ext == "svelte" || ext == "ts" {
                    let content = fs::read_to_string(&path).expect("read file");
                    for (line_no, line) in content.lines().enumerate() {
                        let trimmed = line.trim();
                        if trimmed.starts_with("//") || trimmed.starts_with('*') {
                            continue;
                        }
                        assert!(
                            !trimmed.contains("bg-${") && !trimmed.contains("text-${"),
                            "Dynamic Tailwind class detected at {:?}:{} -> {}",
                            path,
                            line_no + 1,
                            trimmed
                        );
                    }
                }
            }
        }
    }

    check_tailwind_templates(&frontend_src);
}

#[test]
fn test_p16_4_settings_registry_and_brand_about() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("repo root");

    let settings_reg = repo_root.join("frontend/src/lib/settings/registry.ts");
    assert!(
        settings_reg.exists(),
        "frontend/src/lib/settings/registry.ts must exist"
    );
    let reg_content = fs::read_to_string(&settings_reg).expect("read registry.ts");
    assert!(reg_content.contains("registerSettingsSection"));
    assert!(reg_content.contains("getVisibleSettingsSections"));
    assert!(reg_content.contains("about"));

    let about_comp = repo_root.join("frontend/src/lib/components/AboutBrandSection.svelte");
    assert!(
        about_comp.exists(),
        "frontend/src/lib/components/AboutBrandSection.svelte must exist"
    );
}
