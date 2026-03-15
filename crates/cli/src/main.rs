use std::path::PathBuf;

use clap::Parser;
use context_analyzer_engine::collect::collect_project_facts;
use context_analyzer_frontend::load_source_files;
use context_analyzer_report::json::{to_json_compact, to_json_pretty};

#[derive(Debug, Parser)]
#[command(name = "cli")]
#[command(about = "Analyze React context usage from a project folder")]
struct CliArgs {
    #[arg(value_name = "PROJECT_PATH")]
    project_path: PathBuf,

    #[arg(long)]
    pretty: bool,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let cli_args = CliArgs::parse();

    let source_files = load_source_files(&cli_args.project_path)
        .map_err(|error| format!("failed to load source files: {error}"))?;

    let project_facts = collect_project_facts(&source_files);

    let output_json = if cli_args.pretty {
        to_json_pretty(&project_facts)
            .map_err(|error| format!("failed to render pretty json: {error}"))?
    } else {
        to_json_compact(&project_facts)
            .map_err(|error| format!("failed to render json: {error}"))?
    };

    println!("{output_json}");
    Ok(())
}
