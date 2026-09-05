use std::path::{Path, PathBuf};

use chirograph_benchmark::aggregate::aggregate_report;
use chirograph_benchmark::baseline::{
    build_baseline, compare_baseline, compare_baseline_complete, read_baseline, write_baseline,
};
use chirograph_benchmark::corpus::discover_corpus;
use chirograph_benchmark::model::BenchmarkCase;
use chirograph_benchmark::report::{render_human_report, render_json_report};
use chirograph_benchmark::runner::{resolve_chirograph_bin, run_case};
use chirograph_benchmark::selector::select_cases;
use chirograph_benchmark::source::{GitSourceFetcher, refresh_sources, verify_sources};

const HELP: &str = "Chirograph contract benchmark\n\n\
Usage:\n\
  chirograph-benchmark --help\n\
  chirograph-benchmark --list\n\
  chirograph-benchmark all\n\
  chirograph-benchmark SELECTOR\n\
  chirograph-benchmark --verify-sources [SELECTOR]\n\
  chirograph-benchmark --refresh SELECTOR --revision EXACT_SHA\n\
  chirograph-benchmark SELECTOR --baseline PATH\n\
  chirograph-benchmark SELECTOR --write-baseline PATH\n\
  chirograph-benchmark SELECTOR --chirograph-bin PATH\n\
  chirograph-benchmark SELECTOR --format json\n\n\
Selectors:\n\
  all\n\
  REPOSITORY\n\
  scenario:NAME\n\
  REPOSITORY/SCENARIO\n\
  REPOSITORY/SCENARIO/CASE\n";

#[derive(Debug, Clone, PartialEq, Eq)]
enum Command {
    Help,
    List,
    Run(RunOptions),
    VerifySources { selector: String },
    Refresh { selector: String, revision: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RunOptions {
    selector: String,
    baseline: Option<PathBuf>,
    write_baseline: Option<PathBuf>,
    chirograph_bin: Option<PathBuf>,
    format: OutputFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputFormat {
    Human,
    Json,
}

fn main() {
    if let Err(error) = run(std::env::args().skip(1)) {
        eprintln!("chirograph-benchmark: {error}");
        std::process::exit(2);
    }
}

fn run(args: impl IntoIterator<Item = String>) -> Result<(), String> {
    match parse_args(args)? {
        Command::Help => {
            print!("{HELP}");
            Ok(())
        }
        Command::List => {
            let cases = discover()?;
            for case in cases {
                println!("{}", case.id);
            }
            Ok(())
        }
        Command::Run(options) => run_selection(options),
        Command::VerifySources { selector } => verify_selection(&selector),
        Command::Refresh { selector, revision } => refresh_selection(&selector, &revision),
    }
}

fn run_selection(options: RunOptions) -> Result<(), String> {
    let cases = discover()?;
    let selected = select_cases(&cases, &options.selector).map_err(|error| error.to_string())?;
    let chirograph_bin = resolve_chirograph_bin(options.chirograph_bin.as_deref())?;
    let results = selected
        .iter()
        .map(|case| run_case(case, &chirograph_bin))
        .collect::<Vec<_>>();
    let report = aggregate_report(&results);

    let comparison = if let Some(path) = &options.baseline {
        let baseline = read_baseline(path)?;
        Some(if options.selector == "all" {
            compare_baseline_complete(&baseline, &selected, &results)?
        } else {
            compare_baseline(&baseline, &selected, &results)?
        })
    } else {
        None
    };
    if let Some(path) = &options.write_baseline {
        let baseline = build_baseline(&selected, &results)?;
        write_baseline(path, &baseline)?;
    }

    match options.format {
        OutputFormat::Human => print!("{}", render_human_report(&report)),
        OutputFormat::Json => println!(
            "{}",
            render_json_report(&report).map_err(|error| error.to_string())?
        ),
    }

    if let Some(comparison) = comparison {
        for improvement in &comparison.improvements {
            eprintln!("benchmark baseline improvement: {improvement}");
        }
        if !comparison.regressions.is_empty() {
            return Err(format!(
                "benchmark baseline regression:\n{}",
                comparison.regressions.join("\n")
            ));
        }
    }
    Ok(())
}

fn verify_selection(selector: &str) -> Result<(), String> {
    let selected = selected_owned(selector)?;
    let fetcher = GitSourceFetcher::new().map_err(|error| error.to_string())?;
    verify_sources(&selected, &fetcher).map_err(|error| error.to_string())?;
    println!("verified {} benchmark source case(s)", selected.len());
    Ok(())
}

fn refresh_selection(selector: &str, revision: &str) -> Result<(), String> {
    let mut selected = selected_owned(selector)?;
    let fetcher = GitSourceFetcher::new().map_err(|error| error.to_string())?;
    refresh_sources(&mut selected, revision, &fetcher).map_err(|error| error.to_string())?;
    println!(
        "refreshed {} benchmark source case(s) to {revision}",
        selected.len()
    );
    Ok(())
}

fn selected_owned(selector: &str) -> Result<Vec<BenchmarkCase>, String> {
    let cases = discover()?;
    let selected = select_cases(&cases, selector).map_err(|error| error.to_string())?;
    Ok(selected.into_iter().cloned().collect())
}

fn discover() -> Result<Vec<BenchmarkCase>, String> {
    discover_corpus(Path::new("benchmark")).map_err(|error| error.to_string())
}

fn parse_args(args: impl IntoIterator<Item = String>) -> Result<Command, String> {
    let args = args.into_iter().collect::<Vec<_>>();
    let Some(first) = args.first().map(String::as_str) else {
        return Err("missing selector; use --help for usage".to_owned());
    };

    match first {
        "--help" => require_no_extra(&args, Command::Help),
        "--list" => require_no_extra(&args, Command::List),
        "--verify-sources" => parse_verify_sources(&args[1..]),
        "--refresh" => parse_refresh(&args[1..]),
        value if value.starts_with('-') => Err(format!("unknown option: {value}")),
        selector => parse_run(selector, &args[1..]),
    }
}

fn require_no_extra(args: &[String], command: Command) -> Result<Command, String> {
    if args.len() == 1 {
        Ok(command)
    } else {
        Err(format!("unexpected argument: {}", args[1]))
    }
}

fn parse_verify_sources(args: &[String]) -> Result<Command, String> {
    match args {
        [] => Ok(Command::VerifySources {
            selector: "all".to_owned(),
        }),
        [selector] if !selector.starts_with('-') => Ok(Command::VerifySources {
            selector: selector.clone(),
        }),
        [value, ..] => Err(format!("unexpected argument: {value}")),
    }
}

fn parse_refresh(args: &[String]) -> Result<Command, String> {
    let [selector, flag, revision] = args else {
        return Err("--refresh requires SELECTOR --revision EXACT_SHA".to_owned());
    };
    if selector.starts_with('-') || flag != "--revision" || !is_exact_revision(revision) {
        return Err("--refresh requires SELECTOR --revision EXACT_SHA".to_owned());
    }
    Ok(Command::Refresh {
        selector: selector.clone(),
        revision: revision.to_ascii_lowercase(),
    })
}

fn parse_run(selector: &str, args: &[String]) -> Result<Command, String> {
    let mut options = RunOptions {
        selector: selector.to_owned(),
        baseline: None,
        write_baseline: None,
        chirograph_bin: None,
        format: OutputFormat::Human,
    };

    let mut index = 0;
    while index < args.len() {
        let flag = args[index].as_str();
        let value = args
            .get(index + 1)
            .ok_or_else(|| format!("{flag} requires a value"))?;
        match flag {
            "--baseline" => set_once(&mut options.baseline, PathBuf::from(value), flag)?,
            "--write-baseline" => {
                set_once(&mut options.write_baseline, PathBuf::from(value), flag)?;
            }
            "--chirograph-bin" => {
                set_once(&mut options.chirograph_bin, PathBuf::from(value), flag)?;
            }
            "--format" if value == "json" => options.format = OutputFormat::Json,
            "--format" => return Err("--format supports only json".to_owned()),
            _ => return Err(format!("unknown option: {flag}")),
        }
        index += 2;
    }
    if options.baseline.is_some() && options.write_baseline.is_some() {
        return Err("--baseline and --write-baseline are mutually exclusive".to_owned());
    }

    Ok(Command::Run(options))
}

fn set_once<T>(slot: &mut Option<T>, value: T, flag: &str) -> Result<(), String> {
    if slot.is_some() {
        return Err(format!("{flag} may be specified only once"));
    }
    *slot = Some(value);
    Ok(())
}

fn is_exact_revision(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{Command, OutputFormat, parse_args};

    fn parse(args: &[&str]) -> Result<Command, String> {
        parse_args(args.iter().map(|value| (*value).to_owned()))
    }

    #[test]
    fn parses_documented_cli_forms() {
        assert_eq!(parse(&["--help"]), Ok(Command::Help));
        assert_eq!(parse(&["--list"]), Ok(Command::List));
        assert!(matches!(parse(&["all"]), Ok(Command::Run(_))));
        assert!(matches!(parse(&["cargo"]), Ok(Command::Run(_))));
        assert!(matches!(
            parse(&["--verify-sources"]),
            Ok(Command::VerifySources { selector }) if selector == "all"
        ));
        assert!(matches!(
            parse(&["--verify-sources", "cargo"]),
            Ok(Command::VerifySources { selector }) if selector == "cargo"
        ));
        assert!(matches!(
            parse(&[
                "--refresh",
                "cargo/schema-enum-drift/toml-debug-info-spellings",
                "--revision",
                "0123456789abcdef0123456789abcdef01234567",
            ]),
            Ok(Command::Refresh { selector, revision })
                if selector == "cargo/schema-enum-drift/toml-debug-info-spellings"
                    && revision == "0123456789abcdef0123456789abcdef01234567"
        ));
        assert!(matches!(
            parse(&[
                "cargo",
                "--baseline",
                "baseline.json",
                "--chirograph-bin",
                "target/debug/chirograph",
                "--format",
                "json",
            ]),
            Ok(Command::Run(options))
                if options.selector == "cargo"
                    && options.baseline == Some(PathBuf::from("baseline.json"))
                    && options.chirograph_bin == Some(PathBuf::from("target/debug/chirograph"))
                    && options.format == OutputFormat::Json
        ));
    }

    #[test]
    fn rejects_cli_surface_expansion_and_invalid_revision() {
        assert!(parse(&["--all"]).is_err());
        assert!(parse(&["--refresh", "cargo", "--revision", "main"]).is_err());
        assert!(parse(&["all", "--format", "yaml"]).is_err());
        assert!(parse(&["all", "--unknown", "value"]).is_err());
        assert!(
            parse(&[
                "all",
                "--baseline",
                "baseline.json",
                "--write-baseline",
                "next.json",
            ])
            .is_err()
        );
    }
}
