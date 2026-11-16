use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AppConfigFile {
    pub app: AppSection,
    pub brand: BrandSection,
    #[serde(default)]
    pub window: Option<WindowSection>,
    #[serde(default)]
    pub modules: Option<ModulesSection>,
    pub rbac: RbacSection,
    #[serde(default)]
    pub profiles: Option<ProfilesSection>,
    #[serde(default)]
    pub endpoints: Option<EndpointsSection>,
    #[serde(default)]
    pub pro: Option<ProSection>,
    #[serde(default)]
    pub navigation: Option<NavigationSection>,
    #[serde(default)]
    pub integrations: Vec<IntegrationConfig>,
    #[serde(default)]
    pub jobs: Vec<JobConfig>,
    #[serde(default)]
    pub entities: Vec<EntityConfig>,
    #[serde(default)]
    pub menus: Vec<MenuConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AppSection {
    pub name: String,
    pub slug: String,
    pub code: String,
    pub identifier: String,
    pub accent: String,
    pub default_locale: String,
    pub locales: Vec<String>,
    pub platforms: Vec<String>,
    #[serde(default)]
    pub caframework_version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BrandSection {
    pub developer: String,
    #[serde(default)]
    pub company: Option<String>,
    pub website: String,
    pub email: String,
    pub phone: String,
    #[serde(default)]
    pub support_url: Option<String>,
    pub copyright: String,
    pub logo: String,
    #[serde(default = "default_true")]
    pub show_on_lock_screen: Option<bool>,
}

fn default_true() -> Option<bool> {
    Some(true)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WindowSection {
    #[serde(default = "default_window_width")]
    pub width: Option<u32>,
    #[serde(default = "default_window_height")]
    pub height: Option<u32>,
    #[serde(default = "default_min_width")]
    pub min_width: Option<u32>,
    #[serde(default = "default_min_height")]
    pub min_height: Option<u32>,
    #[serde(default = "default_true")]
    pub resizable: Option<bool>,
}

fn default_window_width() -> Option<u32> {
    Some(1200)
}
fn default_window_height() -> Option<u32> {
    Some(800)
}
fn default_min_width() -> Option<u32> {
    Some(900)
}
fn default_min_height() -> Option<u32> {
    Some(600)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ModulesSection {
    #[serde(default = "default_true")]
    pub profiles: Option<bool>,
    #[serde(default = "default_true")]
    pub ai: Option<bool>,
    #[serde(default = "default_true")]
    pub pro: Option<bool>,
    #[serde(default = "default_true")]
    pub feedback: Option<bool>,
    #[serde(default = "default_true")]
    pub updater: Option<bool>,
    #[serde(default = "default_true")]
    pub crash: Option<bool>,
    #[serde(default = "default_true")]
    pub integrations: Option<bool>,
    #[serde(default = "default_true")]
    pub scheduler: Option<bool>,
    #[serde(default)]
    pub support_page: Option<bool>,
    #[serde(default)]
    pub builder: Option<bool>,
    #[serde(default)]
    pub background: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RbacSection {
    pub super_role: String,
    #[serde(default = "default_true")]
    pub allow_runtime_roles: Option<bool>,
    #[serde(default)]
    pub roles: Vec<RoleConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RoleConfig {
    pub key: String,
    pub label_en: String,
    pub label_id: String,
    #[serde(default)]
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProfilesSection {
    #[serde(default)]
    pub pin_required_for: Option<Vec<String>>,
    #[serde(default)]
    pub age_stages: Option<Vec<AgeStageConfig>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AgeStageConfig {
    pub key: String,
    pub min_age: u8,
    pub max_age: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EndpointsSection {
    #[serde(default)]
    pub gcc_base_url: Option<String>,
    #[serde(default)]
    pub updater_manifest_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProSection {
    #[serde(default)]
    pub gcc_product_id: Option<String>,
    #[serde(default = "default_trial_days")]
    pub trial_days: Option<u32>,
    #[serde(default = "default_device_limit")]
    pub device_limit: Option<u32>,
    #[serde(default = "default_offline_grace_days")]
    pub offline_grace_days: Option<u32>,
    #[serde(default = "default_refresh_hours")]
    pub refresh_hours: Option<u32>,
    #[serde(default = "default_currencies")]
    pub currencies: Option<Vec<String>>,
    #[serde(default)]
    pub features: Option<BTreeMap<String, String>>,
    #[serde(default)]
    pub public_keys: Option<Vec<ProPublicKey>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProPublicKey {
    pub kid: String,
    pub ed25519: String,
}

fn default_trial_days() -> Option<u32> {
    Some(14)
}
fn default_device_limit() -> Option<u32> {
    Some(3)
}
fn default_offline_grace_days() -> Option<u32> {
    Some(7)
}
fn default_refresh_hours() -> Option<u32> {
    Some(6)
}
fn default_currencies() -> Option<Vec<String>> {
    Some(vec!["IDR".into(), "USD".into()])
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NavigationSection {
    #[serde(default)]
    pub primary_action: Option<String>,
    #[serde(default)]
    pub collapsed_default: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct IntegrationConfig {
    pub key: String,
    pub kind: String,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub base_url_from: Option<String>,
    #[serde(default)]
    pub user_configurable: Option<bool>,
    #[serde(default = "default_auth")]
    pub auth: Option<String>,
    #[serde(default)]
    pub auth_header: Option<String>,
    #[serde(default = "default_timeout")]
    pub timeout_ms: Option<u64>,
    #[serde(default = "default_retries")]
    pub retries: Option<u32>,
    #[serde(default)]
    pub rate_limit_per_minute: Option<u32>,
    #[serde(default)]
    pub concurrency: Option<u32>,
    #[serde(default)]
    pub allowed_hosts: Option<Vec<String>>,
}

fn default_auth() -> Option<String> {
    Some("none".into())
}
fn default_timeout() -> Option<u64> {
    Some(15000)
}
fn default_retries() -> Option<u32> {
    Some(2)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct JobConfig {
    pub key: String,
    pub schedule: String,
    #[serde(default = "default_true")]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub background_safe: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EntityConfig {
    pub key: String,
    pub label_en: String,
    pub label_id: String,
    #[serde(default)]
    pub natural_key: Option<Vec<String>>,
    pub formats: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MenuConfig {
    pub key: String,
    pub label_en: String,
    pub label_id: String,
    pub icon: String,
    #[serde(default)]
    pub accent: Option<String>,
    pub group: String,
    #[serde(default)]
    pub permission: Option<String>,
}
