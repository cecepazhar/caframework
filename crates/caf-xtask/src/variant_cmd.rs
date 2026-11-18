use caf_template::codegen::{discover_modules, generate_all_with_modules};
use caf_template::variant::{
    diff_variants, generate_variant_matrix, resolve_variant, resolve_variant_from_file,
    validate_all_variants,
};
use clap::Subcommand;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Subcommand, Debug, Clone)]
pub enum VariantCommand {
    /// Validate build variant(s)
    Validate {
        #[arg(short, long)]
        variant: Option<String>,
        #[arg(long)]
        all: bool,
    },
    /// List all defined variants
    List,
    /// Diff two variants or base vs variant
    Diff { a: String, b: String },
    /// Generate module x variant matrix
    Matrix {
        #[arg(long, default_value = "md")]
        format: String,
    },
    /// Build application for a specific variant (leaves working tree clean)
    Build {
        #[arg(short, long)]
        variant: String,
        #[arg(short, long, default_value = "linux")]
        target: String,
        #[arg(long)]
        release: bool,
    },
}

struct WorkingDirectoryGuard {
    saved_files: Vec<(PathBuf, Option<Vec<u8>>)>,
}

impl WorkingDirectoryGuard {
    fn new() -> Self {
        Self {
            saved_files: Vec::new(),
        }
    }

    fn track(&mut self, path: &Path) {
        if !self.saved_files.iter().any(|(p, _)| p == path) {
            let content = std::fs::read(path).ok();
            self.saved_files.push((path.to_path_buf(), content));
        }
    }
}

impl Drop for WorkingDirectoryGuard {
    fn drop(&mut self) {
        for (path, content) in &self.saved_files {
            if let Some(bytes) = content {
                let _ = std::fs::write(path, bytes);
            } else if path.exists() {
                let _ = std::fs::remove_file(path);
            }
        }
    }
}

fn find_variant_path(key: &str, project_root: &Path) -> Option<PathBuf> {
    let direct = project_root.join("variants").join(format!("{}.toml", key));
    if direct.exists() {
        return Some(direct);
    }
    let direct_raw = project_root.join("variants").join(key);
    if direct_raw.exists() {
        return Some(direct_raw);
    }
    // Also search docs/examples/variants
    let ex = project_root
        .join("docs/examples/variants/variants")
        .join(format!("{}.toml", key));
    if ex.exists() {
        return Some(ex);
    }
    None
}

pub fn run_variant_cmd(
    cmd: &VariantCommand,
    project_root: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        VariantCommand::Validate { variant, all } => {
            if *all || variant.is_none() {
                // If variants dir in project root exists, validate it
                let variants_dir = project_root.join("variants");
                let search_root = if variants_dir.exists() {
                    project_root
                } else if project_root
                    .join("docs/examples/variants/variants")
                    .exists()
                {
                    &project_root.join("docs/examples/variants")
                } else {
                    project_root
                };

                let variants = validate_all_variants(search_root).map_err(|errs| {
                    let mut msg = format!("Validation failed with {} error(s):\n", errs.len());
                    for e in errs {
                        msg.push_str(&format!("  {}\n", e));
                    }
                    msg
                })?;

                if variants.is_empty() {
                    println!("No variants found (base app mode).");
                } else {
                    println!("All {} variant(s) validated successfully:", variants.len());
                    for v in &variants {
                        println!(
                            "  - {} ('{}' - {}) [{} modules]",
                            v.key,
                            v.resolved_config.app.name,
                            v.resolved_config.app.identifier,
                            v.enabled_modules.len()
                        );
                    }
                }
            } else if let Some(key) = variant {
                let path = find_variant_path(key, project_root)
                    .ok_or_else(|| format!("variant '{}' not found in variants/", key))?;
                let rv = resolve_variant_from_file(&path, project_root).map_err(|errs| {
                    let mut msg = format!("Validation failed for variant '{}':\n", key);
                    for e in errs {
                        msg.push_str(&format!("  {}\n", e));
                    }
                    msg
                })?;
                println!(
                    "Variant '{}' ({}) validated successfully.",
                    rv.key, rv.resolved_config.app.name
                );
            }
            Ok(())
        }
        VariantCommand::List => {
            let search_root = if project_root.join("variants").exists() {
                project_root
            } else {
                &project_root.join("docs/examples/variants")
            };
            let variants = validate_all_variants(search_root)
                .map_err(|errs| format!("Failed to load variants: {:?}", errs))?;

            if variants.is_empty() {
                println!("No variants defined.");
                return Ok(());
            }

            println!(
                "{:<12} {:<24} {:<32} {:<10} {:<18}",
                "KEY", "NAME", "IDENTIFIER", "ACCENT", "MODULES"
            );
            println!("{}", "-".repeat(96));
            for v in &variants {
                println!(
                    "{:<12} {:<24} {:<32} {:<10} {:<18}",
                    v.key,
                    v.resolved_config.app.name,
                    v.resolved_config.app.identifier,
                    v.resolved_config.app.accent,
                    v.enabled_modules.join(", ")
                );
            }
            Ok(())
        }
        VariantCommand::Diff { a, b } => {
            let v1 = if a == "base" {
                let base_path = project_root.join("app.toml");
                let _base_content = std::fs::read_to_string(&base_path)?;
                let synth_variant = "[variant]\nkey = \"base\"\nextends = \"app.toml\"\n";
                resolve_variant(synth_variant, None, project_root)
                    .map_err(|e| format!("{:?}", e))?
            } else {
                let p1 = find_variant_path(a, project_root)
                    .ok_or_else(|| format!("variant '{}' not found", a))?;
                resolve_variant_from_file(&p1, project_root).map_err(|e| format!("{:?}", e))?
            };

            let p2 = find_variant_path(b, project_root)
                .ok_or_else(|| format!("variant '{}' not found", b))?;
            let v2 =
                resolve_variant_from_file(&p2, project_root).map_err(|e| format!("{:?}", e))?;

            let diff_out = diff_variants(&v1, &v2);
            print!("{}", diff_out);
            Ok(())
        }
        VariantCommand::Matrix { format } => {
            let search_root = if project_root.join("variants").exists() {
                project_root
            } else {
                &project_root.join("docs/examples/variants")
            };
            let variants = validate_all_variants(search_root)
                .map_err(|errs| format!("Failed to load variants: {:?}", errs))?;

            let matrix_str = generate_variant_matrix(&variants, format);
            print!("{}", matrix_str);
            Ok(())
        }
        VariantCommand::Build {
            variant,
            target,
            release,
        } => {
            let var_path = find_variant_path(variant, project_root)
                .ok_or_else(|| format!("variant '{}' not found", variant))?;
            let rv = resolve_variant_from_file(&var_path, project_root)
                .map_err(|e| format!("Failed to resolve variant '{}': {:?}", variant, e))?;

            println!(
                "Building variant '{}' ({}) for target '{}'...",
                rv.key, rv.resolved_config.app.name, target
            );

            let mut guard = WorkingDirectoryGuard::new();

            // Track existing codegen output paths
            let modules = discover_modules(project_root);
            let outputs = generate_all_with_modules(&rv.resolved_config, &modules);
            for out in &outputs {
                let target_path = project_root.join(&out.path);
                guard.track(&target_path);
            }

            // Write variant codegen files into working tree
            for out in outputs {
                let target_path = project_root.join(&out.path);
                if let Some(parent) = target_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(&target_path, &out.content)?;
            }

            // Filter features to those supported by caf-app
            let known_app_features = [
                "mod-profiles",
                "mod-ai",
                "mod-ai-advanced",
                "mod-pro",
                "mod-feedback",
                "mod-updater",
                "mod-crash",
                "mod-integrations",
                "mod-scheduler",
                "mod-support-page",
                "mod-background",
                "builder",
            ];
            let app_features: Vec<_> = rv
                .cargo_features
                .iter()
                .filter(|f| known_app_features.contains(&f.as_str()))
                .cloned()
                .collect();

            // Run cargo compilation command with features
            let features_arg = app_features.join(",");
            let mut cmd = Command::new("cargo");
            if *release {
                cmd.args([
                    "build",
                    "-p",
                    "caf-app",
                    "--release",
                    "--no-default-features",
                ]);
            } else {
                cmd.args(["check", "-p", "caf-app", "--no-default-features"]);
            }
            if !features_arg.is_empty() {
                cmd.arg("--features").arg(&features_arg);
            }
            cmd.current_dir(project_root);

            let status = cmd.status()?;

            // Explicitly drop guard so all files are restored immediately
            drop(guard);

            if !status.success() {
                return Err(format!("Build command exited with status: {}", status).into());
            }

            println!(
                "Variant '{}' built successfully. Working tree restored to clean state.",
                variant
            );
            Ok(())
        }
    }
}
