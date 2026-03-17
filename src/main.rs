mod codegen;
mod parser;
mod tui;

use anyhow::{Context, Result};
use clap::Parser;
use glob::glob;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "capla extractor",
    about = "Parse .b files and emit .c/.h files to use with CertiRocq.",
    version
)]
struct Cli {
    /// Glob patterns or paths to .b files.
    /// Defaults to all *.b files in the current directory.
    #[arg(value_name = "FILE_OR_GLOB")]
    inputs: Vec<String>,

    /// Directory where the generated .c and .h files will be written.
    #[arg(short, long, default_value = ".", value_name = "DIR")]
    output_dir: PathBuf,

    /// Run without the interactive TUI. Requires --prefix and --output.
    #[arg(long, requires = "prefix", requires = "output")]
    non_interactive: bool,

    /// Prefix to prepend to all exported function names (non-interactive mode only).
    #[arg(long, value_name = "PREFIX", requires = "non_interactive")]
    prefix: Option<String>,

    /// Output file stem for the generated .c/.h files (non-interactive mode only).
    #[arg(long, value_name = "STEM", requires = "non_interactive")]
    output: Option<String>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let patterns: Vec<String> = if cli.inputs.is_empty() {
        vec!["*.b".to_string()]
    } else {
        cli.inputs.clone()
    };

    let mut files: Vec<PathBuf> = Vec::new();
    for pat in &patterns {
        let path = PathBuf::from(pat);
        if path.exists() {
            files.push(path);
        } else {
            for entry in glob(pat).with_context(|| format!("Invalid glob: {}", pat))? {
                files.push(entry?);
            }
        }
    }

    files.sort();
    files.dedup();

    if files.is_empty() {
        eprintln!("No .b files found. Pass file paths or glob patterns as arguments.");
        eprintln!("Example: capla_extractor modular_exp.b float_incr.b");
        std::process::exit(1);
    }

    std::fs::create_dir_all(&cli.output_dir)
        .with_context(|| format!("Cannot create output dir: {:?}", cli.output_dir))?;

    if cli.non_interactive {
        run_non_interactive(
            files,
            cli.output_dir,
            cli.prefix.unwrap(),
            cli.output.unwrap(),
        )
    } else {
        tui::run(files, cli.output_dir)
    }
}

fn run_non_interactive(
    files: Vec<PathBuf>,
    output_dir: PathBuf,
    prefix: String,
    output_stem: String,
) -> Result<()> {
    use crate::codegen::{generate, SelectedFunction};
    use crate::parser::parse_b_file;

    let mut selections: Vec<SelectedFunction> = Vec::new();

    for path in &files {
        let src = std::fs::read_to_string(path)
            .with_context(|| format!("Cannot read {:?}", path))?;

        for sig in parse_b_file(&src) {
            let export_name = if prefix.is_empty() {
                sig.name.clone()
            } else {
                format!("{}{}", prefix, sig.name)
            };
            selections.push(SelectedFunction { sig, export_name });
        }
    }

    if selections.is_empty() {
        eprintln!("No function signatures found in the provided files.");
        std::process::exit(1);
    }

    let header_name = format!("{}.h", output_stem);
    let files_out = generate(&selections, &header_name);

    let h_path = output_dir.join(&header_name);
    let c_path = output_dir.join(format!("{}.c", output_stem));

    std::fs::write(&h_path, &files_out.header)
        .with_context(|| format!("Cannot write {:?}", h_path))?;
    std::fs::write(&c_path, &files_out.source)
        .with_context(|| format!("Cannot write {:?}", c_path))?;

    println!(
        "Generated {} function(s) → {:?} and {:?}",
        selections.len(),
        h_path,
        c_path
    );

    Ok(())
}