#![allow(clippy::print_stdout, clippy::print_stderr, clippy::collapsible_if)]

use std::io::Write as _;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};
use drawcraft_format::ExportScope;
use drawcraft_session::Session;
use serde_json::{Value, json};

#[derive(Parser)]
#[command(
    name = "drawcraft",
    version,
    about = "DrawCraft — an infinite-canvas whiteboard and diagramming app",
    long_about = None
)]
struct Cli {
    /// Start in GrokNight (dark) instead of GrokDay (light).
    #[arg(long, global = true)]
    dark: bool,
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Open a document in the desktop app.
    Open { file: PathBuf },
    /// Start a blank document in the desktop app.
    New,
    /// Run any command headlessly and print its JSON result.
    Run {
        /// Command id, e.g. shape.add or view.zoom_to_fit.
        command: String,
        /// Document to open before running the command.
        #[arg(long)]
        file: Option<PathBuf>,
        /// JSON object with command parameters.
        #[arg(long, default_value = "{}")]
        params: String,
        /// Save the document after the command.
        #[arg(long)]
        save: bool,
    },
    /// Export a document to SVG or PNG.
    Export {
        input: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, value_enum, default_value_t = ExportFormat::Svg)]
        format: ExportFormat,
        #[arg(long, value_enum, default_value_t = ScopeArg::Page)]
        scope: ScopeArg,
        #[arg(long, default_value_t = 1.0)]
        scale: f64,
        #[arg(long)]
        transparent: bool,
    },
    /// Print document metadata.
    Info { file: PathBuf },
    /// Run the MCP stdio server.
    Mcp {
        #[arg(long)]
        file: Option<PathBuf>,
    },
    /// List command ids (useful for agents).
    Catalog {
        #[arg(long, default_value = "")]
        filter: String,
    },
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum ExportFormat {
    Svg,
    Png,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum ScopeArg {
    Page,
    Selection,
}

fn main() -> ExitCode {
    let raw: Vec<String> = std::env::args().collect();
    if let Some(result) = legacy_args(&raw) {
        return result;
    }
    let cli = Cli::parse();
    let dark = cli.dark;
    match cli.command {
        None => launch(None, dark),
        Some(Commands::New) => launch(None, dark),
        Some(Commands::Open { file }) => launch(Some(file), dark),
        Some(Commands::Run {
            command,
            file,
            params,
            save,
        }) => run_command(file, &command, &params, save),
        Some(Commands::Export {
            input,
            out,
            format,
            scope,
            scale,
            transparent,
        }) => export_command(&input, &out, format, scope, scale, transparent),
        Some(Commands::Info { file }) => info_command(&file),
        Some(Commands::Mcp { file }) => mcp_command(file),
        Some(Commands::Catalog { filter }) => commands_command(&filter),
    }
}

/// Backwards-compatible flags from the early prototype.
fn legacy_args(args: &[String]) -> Option<ExitCode> {
    if args.iter().any(|arg| arg == "--mcp") {
        let file = args
            .iter()
            .position(|arg| arg == "--file")
            .and_then(|index| args.get(index + 1))
            .map(PathBuf::from);
        return Some(mcp_command(file));
    }
    if let Some(index) = args.iter().position(|arg| arg == "--headless-export") {
        let input = args.get(index + 1).map(PathBuf::from);
        let out = args.get(index + 2).map(PathBuf::from);
        return Some(match (input, out) {
            (Some(input), Some(out)) => {
                let format = if out.extension().and_then(|value| value.to_str()) == Some("png") {
                    ExportFormat::Png
                } else {
                    ExportFormat::Svg
                };
                export_command(&input, &out, format, ScopeArg::Page, 1.0, false)
            }
            _ => usage("--headless-export requires <input> <output>"),
        });
    }
    if let Some(index) = args.iter().position(|arg| arg == "--cli") {
        let Some(command) = args.get(index + 1) else {
            return Some(usage("--cli requires a command id"));
        };
        let params = args
            .iter()
            .position(|arg| arg == "--params")
            .and_then(|index| args.get(index + 1))
            .map(String::as_str)
            .unwrap_or("{}");
        let file = args
            .iter()
            .position(|arg| arg == "--file")
            .and_then(|index| args.get(index + 1))
            .map(PathBuf::from);
        return Some(run_command(file, command, params, false));
    }
    None
}

fn launch(path: Option<PathBuf>, dark: bool) -> ExitCode {
    match drawcraft_app::run_native(path, dark) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => fail(format!("could not start DrawCraft: {error}")),
    }
}

fn run_command(file: Option<PathBuf>, command: &str, params: &str, save: bool) -> ExitCode {
    let params: Value = match serde_json::from_str(params) {
        Ok(Value::Object(map)) => Value::Object(map),
        Ok(_) => return usage("--params must be a JSON object"),
        Err(error) => return usage(format!("invalid --params JSON: {error}")),
    };
    let mut session = Session::new();
    if let Some(file) = file {
        let result = if file.exists() {
            session.open(&file)
        } else {
            session.save_as(&file).map(|_| ())
        };
        if let Err(error) = result {
            return fail(error.to_string());
        }
    }
    match session.execute(command, &params) {
        Ok(value) => {
            match serde_json::to_string_pretty(&value) {
                Ok(text) => out_line(&text),
                Err(error) => return fail(error.to_string()),
            }
            if save {
                if let Err(error) = session.save() {
                    return fail(error.to_string());
                }
            }
            ExitCode::SUCCESS
        }
        Err(error) => fail(error.to_string()),
    }
}

fn export_command(
    input: &PathBuf,
    out: &PathBuf,
    format: ExportFormat,
    scope: ScopeArg,
    scale: f64,
    transparent: bool,
) -> ExitCode {
    let document = match drawcraft_format::load(input) {
        Ok(document) => document,
        Err(error) => return fail(error.to_string()),
    };
    let scope = match scope {
        ScopeArg::Page => ExportScope::Page,
        ScopeArg::Selection => {
            return usage("selection export requires a desktop session");
        }
    };
    let result = match format {
        ExportFormat::Svg => drawcraft_format::export_svg(&document, scope, &[], out, transparent),
        ExportFormat::Png => {
            drawcraft_format::export_png(&document, scope, &[], out, scale, transparent)
        }
    };
    match result {
        Ok(()) => {
            out_line(&format!("exported {}", out.display()));
            ExitCode::SUCCESS
        }
        Err(error) => fail(error.to_string()),
    }
}

fn info_command(file: &PathBuf) -> ExitCode {
    let document = match drawcraft_format::load(file) {
        Ok(document) => document,
        Err(error) => return fail(error.to_string()),
    };
    let value = json!({
        "path": file.display().to_string(),
        "version": document.version,
        "pages": document.pages.iter().map(|page| json!({
            "id": page.id,
            "name": page.name,
            "shapes": page.shapes.len(),
        })).collect::<Vec<_>>(),
        "active_page": document.active_page,
        "shapes": document.shape_count(),
    });
    out_line(&serde_json::to_string_pretty(&value).unwrap_or_else(|_| "{}".to_string()));
    ExitCode::SUCCESS
}

fn mcp_command(file: Option<PathBuf>) -> ExitCode {
    let mut session = Session::new();
    if let Some(file) = file {
        if let Err(error) = session.open(file) {
            return fail(error.to_string());
        }
    }
    match drawcraft_mcp::serve_stdio(&mut session) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => fail(error.to_string()),
    }
}

fn commands_command(filter: &str) -> ExitCode {
    let filter = filter.to_ascii_lowercase();
    for command in drawcraft_command::commands() {
        if !filter.is_empty()
            && !command.id.to_ascii_lowercase().contains(&filter)
            && !command.label.to_ascii_lowercase().contains(&filter)
        {
            continue;
        }
        let params: Vec<String> = command
            .params
            .iter()
            .map(|param| {
                if param.required {
                    format!("{}:{}", param.name, param.kind.json_type())
                } else {
                    format!("[{}:{}]", param.name, param.kind.json_type())
                }
            })
            .collect();
        out_line(&format!(
            "{:<24} {:<38} {}",
            command.id,
            command.label,
            params.join(" ")
        ));
    }
    ExitCode::SUCCESS
}

/// Write a line without panicking when the consumer closes the pipe.
fn out_line(text: &str) {
    let mut stdout = std::io::stdout();
    let _ = writeln!(stdout, "{text}");
}

fn usage(message: impl Into<String>) -> ExitCode {
    eprintln!("{}", message.into());
    ExitCode::from(2)
}

fn fail(message: impl Into<String>) -> ExitCode {
    eprintln!("error: {}", message.into());
    ExitCode::FAILURE
}
