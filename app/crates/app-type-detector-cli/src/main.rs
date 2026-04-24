use std::path::PathBuf;
use std::process::ExitCode;

use app_type_detector::{
    default_rules::default_ruleset, detect_with, render_human_readable, DetectionReport,
    FilesystemSnapshot, Ruleset, SynthesisConfig,
};
use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser, Debug)]
#[command(name = "app-type-detector", version, about = "Classify any codebase.")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Detect the type of a project directory.
    Detect {
        /// Project directory. Defaults to `.`.
        path: Option<PathBuf>,
        /// Output format.
        #[arg(long, value_enum, default_value_t = Format::Text)]
        format: Format,
        /// Path to a custom ruleset JSON. When omitted, the bundled default is used.
        #[arg(long)]
        rules: Option<PathBuf>,
        /// Strip evidence arrays from scorecard fires for compact output.
        #[arg(long)]
        no_evidence: bool,
        /// Override the synthesizer dominance margin (default 1.5).
        #[arg(long)]
        margin: Option<f32>,
    },
}

#[derive(ValueEnum, Debug, Clone, Copy)]
enum Format {
    Json,
    Text,
    Tsv,
    FiresJsonl,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Detect {
            path,
            format,
            rules,
            no_evidence,
            margin,
        } => {
            let path = path.unwrap_or_else(|| PathBuf::from("."));
            let ruleset = match rules {
                Some(p) => match std::fs::read_to_string(&p) {
                    Ok(text) => match Ruleset::from_json(&text) {
                        Ok(r) => r,
                        Err(e) => {
                            eprintln!("failed to parse --rules {}: {}", p.display(), e);
                            return ExitCode::from(2);
                        }
                    },
                    Err(e) => {
                        eprintln!("failed to read --rules {}: {}", p.display(), e);
                        return ExitCode::from(2);
                    }
                },
                None => default_ruleset().clone(),
            };

            let snap = match FilesystemSnapshot::new(&path) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("failed to snapshot {}: {}", path.display(), e);
                    return ExitCode::from(2);
                }
            };

            let mut config = SynthesisConfig::default();
            if let Some(m) = margin {
                config.dominance_margin = m;
            }

            let mut report = detect_with(&snap, &ruleset, &config);
            if no_evidence {
                strip_evidence(&mut report);
            }

            match format {
                Format::Json => println!("{}", report.to_json()),
                Format::Text => println!("{}", render_human_readable(&report)),
                Format::Tsv => println!("{}", report.to_tsv()),
                Format::FiresJsonl => print!("{}", report.scorecard.fires_jsonl()),
            }

            ExitCode::SUCCESS
        }
    }
}

fn strip_evidence(report: &mut DetectionReport) {
    for fire in report.scorecard.fires.iter_mut() {
        fire.evidence.clear();
    }
}
