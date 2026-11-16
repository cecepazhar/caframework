use caf_template::validation::validate_app_toml;
use proptest::prelude::*;

const BASE_VALID_TOML: &str = r#"
[app]
name = "My App"
slug = "myapp"
code = "MYA"
identifier = "com.example.myapp"
accent = "indigo"
default_locale = "id"
locales = ["id", "en"]
platforms = ["linux", "windows"]

[brand]
developer = "Cecep Saeful Azhar"
website = "https://www.cecepazhar.com"
email = "hi@cecepazhar.com"
phone = "+6285220696117"
copyright = "© 2026 Cecep"
logo = "assets/logo.svg"

[rbac]
super_role = "super_admin"

[[rbac.roles]]
key = "super_admin"
label_en = "Super admin"
label_id = "Super admin"
permissions = ["*"]

[[menus]]
key = "notes"
label_en = "Notes"
label_id = "Catatan"
icon = "M3 7h18M3 12h18M3 17h18"
group = "daily"
"#;

#[test]
fn test_rule1_unknown_key_rejected_with_location() {
    let toml_str = format!("{}\nunknown_root_key = 123\n", BASE_VALID_TOML);
    let errs = validate_app_toml(&toml_str, None).expect_err("unknown key must fail");
    assert!(!errs.is_empty());
    assert!(
        errs[0].message.contains("unknown field"),
        "Expected unknown field message, got: {:?}",
        errs[0]
    );
    assert!(errs[0].line.is_some(), "Line must be reported");
    assert!(errs[0].column.is_some(), "Column must be reported");
}

#[test]
fn test_rule2_app_slug_code_identifier_accent_locales() {
    // Bad slug
    let bad_slug =
        BASE_VALID_TOML.replace("slug = \"myapp\"", "slug = \"Invalid_Slug_With_Upper\"");
    let errs = validate_app_toml(&bad_slug, None).expect_err("bad slug must fail");
    assert!(errs.iter().any(|e| e.path == "app.slug"));

    // Bad code
    let bad_code = BASE_VALID_TOML.replace("code = \"MYA\"", "code = \"TOOLONGARE\"");
    let errs = validate_app_toml(&bad_code, None).expect_err("bad code must fail");
    assert!(errs.iter().any(|e| e.path == "app.code"));

    // Bad accent
    let bad_accent = BASE_VALID_TOML.replace("accent = \"indigo\"", "accent = \"chartreuse\"");
    let errs = validate_app_toml(&bad_accent, None).expect_err("bad accent must fail");
    assert!(errs.iter().any(|e| e.path == "app.accent"));

    // Bad default_locale
    let bad_loc = BASE_VALID_TOML.replace("default_locale = \"id\"", "default_locale = \"fr\"");
    let errs = validate_app_toml(&bad_loc, None).expect_err("bad default_locale must fail");
    assert!(errs.iter().any(|e| e.path == "app.default_locale"));
}

#[test]
fn test_rule3_brand_rules() {
    // Missing developer
    let bad_dev =
        BASE_VALID_TOML.replace("developer = \"Cecep Saeful Azhar\"", "developer = \"   \"");
    let errs = validate_app_toml(&bad_dev, None).expect_err("empty developer must fail");
    assert!(errs.iter().any(|e| e.path == "brand.developer"));

    // Non-https website
    let bad_web = BASE_VALID_TOML.replace(
        "website = \"https://www.cecepazhar.com\"",
        "website = \"http://insecure.com\"",
    );
    let errs = validate_app_toml(&bad_web, None).expect_err("http website must fail");
    assert!(errs.iter().any(|e| e.path == "brand.website"));

    // Invalid email
    let bad_email =
        BASE_VALID_TOML.replace("email = \"hi@cecepazhar.com\"", "email = \"notanemail\"");
    let errs = validate_app_toml(&bad_email, None).expect_err("bad email must fail");
    assert!(errs.iter().any(|e| e.path == "brand.email"));

    // Non-E164 phone
    let bad_phone =
        BASE_VALID_TOML.replace("phone = \"+6285220696117\"", "phone = \"0852-2069-6117\"");
    let errs = validate_app_toml(&bad_phone, None).expect_err("bad phone must fail");
    assert!(errs.iter().any(|e| e.path == "brand.phone"));
}

#[test]
fn test_rule4_rbac_rules() {
    // Super role missing ["*"]
    let bad_super =
        BASE_VALID_TOML.replace("permissions = [\"*\"]", "permissions = [\"notes:view\"]");
    let errs = validate_app_toml(&bad_super, None).expect_err("super role without * must fail");
    assert!(errs.iter().any(|e| e.path == "rbac.super_role"));

    // Duplicate role
    let dup_role = format!(
        "{}\n[[rbac.roles]]\nkey = \"super_admin\"\nlabel_en = \"Super\"\nlabel_id = \"Super\"\npermissions = [\"*\"]\n",
        BASE_VALID_TOML
    );
    let errs = validate_app_toml(&dup_role, None).expect_err("duplicate role must fail");
    assert!(
        errs.iter()
            .any(|e| e.path == "rbac.roles.key" && e.message.contains("duplicate"))
    );

    // Unknown resource permission
    let unknown_perm = format!(
        "{}\n[[rbac.roles]]\nkey = \"editor\"\nlabel_en = \"Editor\"\nlabel_id = \"Editor\"\npermissions = [\"unknown_widget:view\"]\n",
        BASE_VALID_TOML
    );
    let errs = validate_app_toml(&unknown_perm, None).expect_err("unknown resource must fail");
    assert!(
        errs.iter()
            .any(|e| e.path == "rbac.roles.permissions" && e.message.contains("unknown resource"))
    );
}

#[test]
fn test_rule5_job_schedule_cron() {
    let bad_cron = format!(
        "{}\n[[jobs]]\nkey = \"backup.bad\"\nschedule = \"99 99 * * *\"\nenabled = true\n",
        BASE_VALID_TOML
    );
    let errs = validate_app_toml(&bad_cron, None).expect_err("invalid cron must fail");
    assert!(errs.iter().any(|e| e.path == "jobs.schedule"));
}

#[test]
fn test_rule6_endpoints_https() {
    let bad_ep = format!(
        "{}\n[endpoints]\ngcc_base_url = \"http://external-insecure-site.com\"\n",
        BASE_VALID_TOML
    );
    let errs = validate_app_toml(&bad_ep, None).expect_err("http endpoint must fail");
    assert!(errs.iter().any(|e| e.path == "endpoints.gcc_base_url"));
}

#[test]
fn test_rule7_menus_reserved_and_length() {
    let reserved_menu = BASE_VALID_TOML.replace("key = \"notes\"", "key = \"settings\"");
    let errs = validate_app_toml(&reserved_menu, None).expect_err("reserved menu key must fail");
    assert!(
        errs.iter()
            .any(|e| e.path == "menus.key" && e.message.contains("reserved"))
    );
}

#[test]
fn test_rule8_backup_under_modules_rejected() {
    let bad_backup = format!("{}\n[modules]\nbackup = true\n", BASE_VALID_TOML);
    let errs = validate_app_toml(&bad_backup, None).expect_err("backup in modules must fail");
    assert!(
        errs.iter()
            .any(|e| e.path == "modules.backup" && e.message.contains("backup is mandatory"))
    );
}

#[test]
fn test_all_five_errors_reported_in_one_run() {
    // 1. bad slug
    // 2. bad code
    // 3. non-https website
    // 4. invalid email
    // 5. reserved menu key
    let bad_toml = BASE_VALID_TOML
        .replace("slug = \"myapp\"", "slug = \"BAD_SLUG!\"")
        .replace("code = \"MYA\"", "code = \"TOOLONGARE\"")
        .replace(
            "website = \"https://www.cecepazhar.com\"",
            "website = \"http://insecure.com\"",
        )
        .replace("email = \"hi@cecepazhar.com\"", "email = \"bad-email\"")
        .replace("key = \"notes\"", "key = \"settings\"");

    let errs = validate_app_toml(&bad_toml, None).expect_err("five errors must all be returned");
    assert!(
        errs.len() >= 5,
        "Expected at least 5 errors reported at once, got {}: {:?}",
        errs.len(),
        errs
    );

    let paths: Vec<&str> = errs.iter().map(|e| e.path.as_str()).collect();
    assert!(paths.contains(&"app.slug"), "Missing app.slug error");
    assert!(paths.contains(&"app.code"), "Missing app.code error");
    assert!(
        paths.contains(&"brand.website"),
        "Missing brand.website error"
    );
    assert!(paths.contains(&"brand.email"), "Missing brand.email error");
    assert!(paths.contains(&"menus.key"), "Missing menus.key error");

    for e in &errs {
        assert!(e.line.is_some(), "Error {:?} must have line number", e);
        assert!(e.column.is_some(), "Error {:?} must have column number", e);
    }
}

// Proptest over random inputs never panics (>= 10,000 cases in CI / proptest)
proptest! {
    #![proptest_config(ProptestConfig::with_cases(10000))]
    #[test]
    fn proptest_random_toml_never_panics(s in "\\PC{0,200}") {
        let _ = validate_app_toml(&s, None);
    }
}
