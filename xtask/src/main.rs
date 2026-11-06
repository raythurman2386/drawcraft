#![allow(clippy::print_stdout, clippy::print_stderr)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let command = args.first().map(String::as_str).unwrap_or("help");
    match dispatch(command, &args[1..]) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("xtask: {error}");
            ExitCode::FAILURE
        }
    }
}

fn dispatch(command: &str, args: &[String]) -> Result<()> {
    match command {
        "fmt" => run_cargo(&["fmt", "--all"]),
        "clippy" => run_cargo(&[
            "clippy",
            "--workspace",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ]),
        "test" => run_cargo(&["test", "--workspace"]),
        "build" => run_cargo(&["build", "--workspace"]),
        "wasm" => wasm(),
        "bundle" => bundle(),
        "layers" | "check-layers" => check_layers(),
        "commands" | "check-commands" => check_commands(),
        "fuzz" => fuzz(args),
        "check" | "all" => {
            run_cargo(&["fmt", "--all", "--", "--check"])?;
            run_cargo(&[
                "clippy",
                "--workspace",
                "--all-targets",
                "--",
                "-D",
                "warnings",
            ])?;
            run_cargo(&["test", "--workspace"])?;
            check_layers()?;
            check_commands()?;
            println!("all checks passed");
            Ok(())
        }
        "run" => {
            let mut command = Command::new("cargo");
            command.args(["run", "-p", "drawcraft-app", "--"]);
            command.args(args);
            status(command)
        }
        "help" | "--help" | "-h" => {
            println!(
                "xtask: fmt | clippy | test | build | wasm | bundle | layers | commands | fuzz [secs] | check | all"
            );
            Ok(())
        }
        other => Err(format!("unknown task {other}").into()),
    }
}

fn run_cargo(args: &[&str]) -> Result<()> {
    let mut command = Command::new("cargo");
    command.args(args);
    status(command)
}

fn status(mut command: Command) -> Result<()> {
    let status = command.status()?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("command failed with {status}").into())
    }
}

fn workspace_root() -> Result<PathBuf> {
    Ok(Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("workspace root not found")?
        .to_path_buf())
}

fn metadata() -> Result<serde_json::Value> {
    let output = Command::new("cargo")
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .current_dir(workspace_root()?)
        .output()?;
    if !output.status.success() {
        return Err("cargo metadata failed".into());
    }
    Ok(serde_json::from_slice(&output.stdout)?)
}

fn check_layers() -> Result<()> {
    let metadata = metadata()?;
    let allowed = allowed_edges();
    let packages = metadata
        .get("packages")
        .and_then(serde_json::Value::as_array)
        .ok_or("metadata packages missing")?;
    let mut problems = Vec::new();
    for package in packages {
        let Some(name) = package.get("name").and_then(serde_json::Value::as_str) else {
            continue;
        };
        let Some(allowed_for_crate) = allowed.get(name) else {
            continue;
        };
        let Some(dependencies) = package
            .get("dependencies")
            .and_then(serde_json::Value::as_array)
        else {
            continue;
        };
        for dependency in dependencies {
            let Some(dep_name) = dependency.get("name").and_then(serde_json::Value::as_str) else {
                continue;
            };
            if !dep_name.starts_with("drawcraft-") {
                continue;
            }
            if !allowed_for_crate.contains(dep_name) {
                problems.push(format!(
                    "{name} must not depend on {dep_name} (layering violation)"
                ));
            }
        }
    }
    if problems.is_empty() {
        println!("layering ok ({} workspace crates)", packages.len());
        Ok(())
    } else {
        for problem in &problems {
            eprintln!("{problem}");
        }
        Err(format!("{} layering violation(s)", problems.len()).into())
    }
}

fn allowed_edges() -> BTreeMap<&'static str, BTreeSet<&'static str>> {
    let mut allowed: BTreeMap<&'static str, BTreeSet<&'static str>> = BTreeMap::new();
    allowed.insert("drawcraft-geometry", BTreeSet::new());
    allowed.insert("drawcraft-command", BTreeSet::new());
    allowed.insert("drawcraft-document", BTreeSet::from(["drawcraft-geometry"]));
    allowed.insert(
        "drawcraft-editor",
        BTreeSet::from([
            "drawcraft-command",
            "drawcraft-document",
            "drawcraft-geometry",
        ]),
    );
    allowed.insert(
        "drawcraft-render",
        BTreeSet::from(["drawcraft-document", "drawcraft-geometry"]),
    );
    allowed.insert(
        "drawcraft-format",
        BTreeSet::from([
            "drawcraft-document",
            "drawcraft-geometry",
            "drawcraft-render",
        ]),
    );
    allowed.insert(
        "drawcraft-session",
        BTreeSet::from([
            "drawcraft-command",
            "drawcraft-document",
            "drawcraft-editor",
            "drawcraft-format",
            "drawcraft-geometry",
            "drawcraft-render",
        ]),
    );
    allowed.insert(
        "drawcraft-mcp",
        BTreeSet::from(["drawcraft-command", "drawcraft-format", "drawcraft-session"]),
    );
    allowed.insert(
        "drawcraft-app",
        BTreeSet::from([
            "drawcraft-command",
            "drawcraft-document",
            "drawcraft-editor",
            "drawcraft-format",
            "drawcraft-geometry",
            "drawcraft-mcp",
            "drawcraft-render",
            "drawcraft-session",
        ]),
    );
    allowed.insert("xtask", BTreeSet::new());
    allowed
}

fn check_commands() -> Result<()> {
    run_cargo(&[
        "test",
        "-p",
        "drawcraft-editor",
        "--lib",
        "every_editor_command_is_handled_by_dispatch",
        "--",
    ])?;
    run_cargo(&[
        "test",
        "-p",
        "drawcraft-session",
        "--lib",
        "every_session_command_is_handled_by_execute",
        "--",
    ])
}

fn fuzz(args: &[String]) -> Result<()> {
    let secs = args
        .first()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(30);
    let root = workspace_root()?;
    let fuzz_dir = root.join("fuzz");
    if !fuzz_dir.is_dir() {
        return Err("fuzz/ directory missing".into());
    }
    let nightly = Command::new("rustup")
        .args(["run", "nightly", "rustc", "--version"])
        .status();
    if !matches!(nightly, Ok(status) if status.success()) {
        return Err(
            "xtask fuzz requires the nightly toolchain (rustup toolchain install nightly)".into(),
        );
    }
    let cargo_fuzz = Command::new("cargo")
        .args(["+nightly", "fuzz", "-V"])
        .status();
    if !matches!(cargo_fuzz, Ok(status) if status.success()) {
        return Err("xtask fuzz requires cargo-fuzz (cargo install cargo-fuzz --locked)".into());
    }
    // cargo-fuzz defaults --target to its own compile-time triple. Musl-static
    // installs (e.g. cargo-binstall / taiki-e) would otherwise try ASAN+musl.
    let host = nightly_host_triple()?;
    let max_total_time = format!("-max_total_time={secs}");
    let mut command = Command::new("cargo");
    command.current_dir(&fuzz_dir).args([
        "+nightly",
        "fuzz",
        "run",
        "from_json",
        "--target",
        &host,
        "--",
        &max_total_time,
    ]);
    status(command)
}

fn nightly_host_triple() -> Result<String> {
    let output = Command::new("rustup")
        .args(["run", "nightly", "rustc", "-vV"])
        .output()?;
    if !output.status.success() {
        return Err("failed to query nightly rustc host triple".into());
    }
    let text = String::from_utf8_lossy(&output.stdout);
    text.lines()
        .find_map(|line| line.strip_prefix("host: ").map(str::to_owned))
        .ok_or_else(|| "nightly rustc -vV missing host line".into())
}

fn wasm() -> Result<()> {
    let targets = ["wasm32-unknown-unknown"];
    let installed = Command::new("rustup")
        .args(["target", "list", "--installed"])
        .output();
    if let Ok(output) = installed {
        let text = String::from_utf8_lossy(&output.stdout);
        if !text.lines().any(|line| line.trim() == targets[0]) {
            return Err("wasm32-unknown-unknown is not installed (rustup target add wasm32-unknown-unknown)".into());
        }
    }
    for package in [
        "drawcraft-geometry",
        "drawcraft-command",
        "drawcraft-document",
        "drawcraft-editor",
    ] {
        run_cargo(&["build", "--target", "wasm32-unknown-unknown", "-p", package])?;
    }
    println!("wasm core crates built");
    Ok(())
}

fn bundle() -> Result<()> {
    run_cargo(&["build", "--release", "-p", "drawcraft-app"])?;
    let root = workspace_root()?;
    let binary = root.join("target").join("release").join(binary_name());
    if binary.exists() {
        println!("bundled binary: {}", binary.display());
        #[cfg(target_os = "macos")]
        bundle_macos_icons(&root)?;
        Ok(())
    } else {
        Err(format!("expected binary at {}", binary.display()).into())
    }
}

fn binary_name() -> &'static str {
    if cfg!(windows) {
        "drawcraft.exe"
    } else {
        "drawcraft"
    }
}

#[cfg(target_os = "macos")]
fn bundle_macos_icons(root: &std::path::Path) -> Result<()> {
    use std::fs;

    let bundle = root.join("target").join("release").join("DrawCraft.app");
    let resources = bundle.join("Contents").join("Resources");
    fs::create_dir_all(bundle.join("Contents").join("MacOS"))?;
    fs::create_dir_all(&resources)?;
    fs::copy(
        root.join("assets").join("logo").join("drawcraft.icns"),
        resources.join("DrawCraft.icns"),
    )?;
    fs::copy(
        root.join("target").join("release").join("drawcraft"),
        bundle.join("Contents").join("MacOS").join("drawcraft"),
    )?;
    fs::write(
        bundle.join("Contents").join("Info.plist"),
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "https://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>CFBundleName</key><string>DrawCraft</string>
  <key>CFBundleDisplayName</key><string>DrawCraft</string>
  <key>CFBundleExecutable</key><string>drawcraft</string>
  <key>CFBundleIdentifier</key><string>app.drawcraft.desktop</string>
  <key>CFBundleIconFile</key><string>DrawCraft</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>{env!("CARGO_PKG_VERSION")}</string>
  <key>NSHighResolutionCapable</key><true/>
</dict></plist>
"#
        ),
    )?;
    println!(
        "bundled app icon: {}",
        resources.join("DrawCraft.icns").display()
    );
    Ok(())
}
