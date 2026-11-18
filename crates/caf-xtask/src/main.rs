use caf_template::config::AppConfigFile;
use caf_template::validation::validate_app_toml;
use clap::{Parser, Subcommand};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(
    name = "xtask",
    about = "CAFramework internal developer and scaffolding tool"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Generate application icons from base SVG
    GenerateIcons,

    /// Codegen frontend and Rust files from app.toml
    Codegen {
        #[arg(short, long, default_value = "app.toml")]
        config: PathBuf,
        #[arg(long)]
        check: bool,
    },
    /// Scaffold a new app from CAFramework base template
    NewApp {
        #[arg(short, long, default_value = "app.toml")]
        config: PathBuf,
        #[arg(short, long)]
        out: PathBuf,
        #[arg(long)]
        with_sample: bool,
        #[arg(long)]
        dry_run: bool,
    },
    /// Run CI quality and architecture guards
    Guard {
        #[arg(long)]
        only: Option<String>,
    },
    /// Validate app.toml config schema (preferred)
    Validate {
        #[arg(short, long, default_value = "app.toml")]
        config: PathBuf,
    },
    /// Validate app.toml config schema (alias for validate, deprecated)
    Check {
        #[arg(short, long, default_value = "app.toml")]
        config: PathBuf,
    },
    /// Generate JSON schema for app.toml
    Schema {
        #[arg(short, long, default_value = "docs/contracts/app-toml.schema.json")]
        out: PathBuf,
    },
    /// Run local GCC Billing Hub mock server
    GccMock {
        #[arg(short, long, default_value_t = 8888)]
        port: u16,
    },
    /// Manage build variants (A-02)
    Variant {
        #[command(subcommand)]
        command: variant_cmd::VariantCommand,
    },
}

fn load_and_validate_config(path: &Path) -> Result<AppConfigFile, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)?;
    let root = path.parent().unwrap_or_else(|| Path::new("."));
    match validate_app_toml(&content, Some(root)) {
        Ok(cfg) => Ok(cfg),
        Err(errs) => {
            let mut msg = format!("Validation failed with {} error(s):\n", errs.len());
            for e in errs {
                msg.push_str(&format!("  {}\n", e));
            }
            Err(msg.into())
        }
    }
}

fn run_validate(config_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let cfg = load_and_validate_config(config_path)?;
    println!("app.toml validation succeeded for {}", cfg.app.name);
    Ok(())
}

fn run_schema(out_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let schema_json = caf_template::schema::generate_app_config_schema();
    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(out_path, schema_json)?;
    println!("JSON Schema successfully written to {:?}", out_path);
    Ok(())
}

fn run_codegen(config_path: &Path, check: bool) -> Result<(), Box<dyn std::error::Error>> {
    let cfg = load_and_validate_config(config_path)?;
    let root = config_path.parent().unwrap_or_else(|| Path::new("."));

    // 1. Generate TS Bindings (commands.rs)
    if !check {
        crate::codegen::ts_bindings::generate_bindings(root)?;
    }

    // 2. Run caf-template codegen
    caf_template::codegen::run_codegen_on_root(root, &cfg, check)?;

    if check {
        println!("Codegen check passed: all generated files are up to date.");
    } else {
        println!("Codegen completed successfully for {}", cfg.app.name);
    }
    Ok(())
}

fn run_generate_icons() -> Result<(), Box<dyn std::error::Error>> {
    println!("Generating icons from app-icon.svg (stub)");
    Ok(())
}

fn run_new_app(
    config_path: &Path,
    out_dir: &Path,
    with_sample: bool,
    dry_run: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let cfg = load_and_validate_config(config_path)?;
    println!(
        "Scaffolding new app '{}' ({}) into {:?}",
        cfg.app.name, cfg.app.slug, out_dir
    );

    if out_dir.exists() && out_dir.read_dir()?.next().is_some() {
        return Err(format!("Target directory {:?} is not empty!", out_dir).into());
    }

    if dry_run {
        println!("Dry run mode enabled. No files written.");
        return Ok(());
    }

    let template_root = match std::env::var("CARGO_MANIFEST_DIR") {
        Ok(dir) => {
            let p = PathBuf::from(dir);
            p.parent()
                .and_then(|p| p.parent())
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| PathBuf::from("."))
        }
        Err(_) => std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
    };
    fs::create_dir_all(out_dir)?;

    let output = std::process::Command::new("git")
        .arg("ls-files")
        .current_dir(&template_root)
        .output()?;

    let files_list = String::from_utf8_lossy(&output.stdout);

    for rel_str in files_list.lines() {
        if rel_str.is_empty() {
            continue;
        }

        let path = template_root.join(rel_str);

        if rel_str.starts_with(".git")
            || rel_str.starts_with("target")
            || rel_str.starts_with("dist/")
            || rel_str.starts_with("gen/")
            || rel_str.starts_with("build/bin/")
            || rel_str.starts_with("frontend/node_modules")
            || rel_str.starts_with("frontend/.svelte-kit")
            || rel_str.starts_with("frontend/build")
            || rel_str.contains("evidence")
        {
            continue;
        }

        if !with_sample
            && (rel_str.contains("sample") || rel_str.contains("fixture"))
            && !rel_str.contains("guard_tests")
        {
            continue;
        }

        let dest = out_dir.join(rel_str);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }

        if path.is_file() {
            let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if file_name.ends_with(".rs")
                || file_name.ends_with(".toml")
                || file_name.ends_with(".json")
                || file_name.ends_with(".svelte")
                || file_name.ends_with(".ts")
                || file_name.ends_with(".md")
            {
                let content = fs::read_to_string(&path)?;
                let replaced = content
                    .replace("CAFramework", &cfg.app.name)
                    .replace("caframework", &cfg.app.slug)
                    .replace("com.fathforce.caframework", &cfg.app.identifier)
                    .replace("CAF", &cfg.app.code);
                fs::write(&dest, replaced)?;
            } else {
                fs::copy(&path, &dest)?;
            }
        }
    }

    let build_dir = out_dir.join("frontend/build");
    fs::create_dir_all(&build_dir)?;
    if !build_dir.join("index.html").exists() {
        fs::write(
            build_dir.join("index.html"),
            "<!DOCTYPE html><html><head><title>CAFramework</title></head><body><div id=\"app\"></div></body></html>",
        )?;
    }

    fs::copy(config_path, out_dir.join("app.toml"))?;
    run_codegen(&out_dir.join("app.toml"), false)?;

    println!("Successfully scaffolded {} at {:?}", cfg.app.name, out_dir);
    Ok(())
}

mod codegen;
pub mod gcc_mock;
mod guard;
mod scanner;
pub mod variant_cmd;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    match cli.command {
        Commands::GenerateIcons => run_generate_icons(),
        Commands::Validate { config } => run_validate(&config),
        Commands::Check { config } => {
            eprintln!("warning: 'check' is deprecated; use 'validate' instead.");
            run_validate(&config)
        }
        Commands::Schema { out } => run_schema(&out),
        Commands::Codegen { config, check } => run_codegen(&config, check),
        Commands::NewApp {
            config,
            out,
            with_sample,
            dry_run,
        } => run_new_app(&config, &out, with_sample, dry_run),
        Commands::Guard { only } => guard::run_guard(only.as_deref()),
        Commands::GccMock { port } => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(gcc_mock::run_mock_server(port))
        }
        Commands::Variant { command } => {
            let root = std::env::current_dir()?;
            variant_cmd::run_variant_cmd(&command, &root)
        }
    }
}
