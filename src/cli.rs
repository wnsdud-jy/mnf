use std::path::PathBuf;

use anyhow::Result;
use clap::Args;
use indicatif::{ProgressBar, ProgressStyle};
use owo_colors::OwoColorize;
use tokio_util::sync::CancellationToken;

use crate::{
    checker::MojangChecker,
    model::{SearchEvent, SearchOptions, SearchProgress, SearchStopReason, SearchSummary},
    output::save_results,
    search::run_search,
    validation::validate_search_options,
};

#[derive(Clone, Debug, Args)]
pub struct SearchArgs {
    #[arg(long)]
    pub length: u8,
    #[arg(long = "starts-with", default_value = "")]
    pub starts_with: String,
    #[arg(long, default_value_t = 20)]
    pub results: usize,
    #[arg(long, default_value_t = 500)]
    pub max_checks: usize,
    #[arg(long)]
    pub save: Option<PathBuf>,
}

pub async fn run_cli_search(args: SearchArgs) -> Result<()> {
    let options = validate_search_options(
        args.length,
        &args.starts_with,
        args.results,
        args.max_checks,
    )?;
    let checker = MojangChecker::new()?;
    let save_path = args.save.clone();
    let mut hits = Vec::new();
    let progress = build_progress_bar(&options);

    print_header(&options, save_path.as_ref());

    let summary = run_search(
        options.clone(),
        &checker,
        CancellationToken::new(),
        |event| match event {
            SearchEvent::Progress(state) => update_progress(&progress, &state, &options),
            SearchEvent::Hit(name) => {
                hits.push(name.clone());
                progress.println(format!(
                    "{} {} {}",
                    "[hit]".black().on_bright_green().bold(),
                    name.bright_green().bold(),
                    "likely available".bright_black()
                ));
            }
            SearchEvent::Finished(state) => finish_progress(&progress, &state),
        },
    )
    .await?;

    if summary.progress.found == 0 {
        println!(
            "{} No likely-available names found in the checked range.",
            "[note]".black().on_yellow().bold()
        );
    }

    if let Some(path) = save_path.as_ref() {
        save_results(path, &hits)?;
        println!(
            "{} {} {}",
            "[save]".black().on_bright_blue().bold(),
            path.display().to_string().bright_blue().bold(),
            format!("({} result(s))", hits.len()).bright_black()
        );
    }

    print_summary(&summary);

    Ok(())
}

fn build_progress_bar(options: &SearchOptions) -> ProgressBar {
    let bar = ProgressBar::new(options.max_checks as u64);
    let style = ProgressStyle::with_template(
        "{spinner:.cyan} [{elapsed_precise}] [{bar:32}] {pos:>4}/{len:4} {msg}",
    )
    .expect("valid progress style")
    .progress_chars("=>-");
    bar.set_style(style);
    bar.enable_steady_tick(std::time::Duration::from_millis(120));
    bar.set_message(format!(
        "hits 0/{} | prefix {}",
        options.results,
        if options.prefix.is_empty() {
            "-".to_string()
        } else {
            options.prefix.clone()
        }
    ));
    bar
}

fn print_header(options: &SearchOptions, save_path: Option<&PathBuf>) {
    println!("\n{}", "MNF / mission control".bright_cyan().bold());
    println!(
        "{}",
        "Public Mojang lookup. Results are labeled likely available.".bright_black()
    );
    println!(
        "{} {} | {} {} | {} {} | {} {}",
        "len".bright_black().bold(),
        options.length.to_string().bright_white().bold(),
        "prefix".bright_black().bold(),
        if options.prefix.is_empty() {
            "-".bright_white().to_string()
        } else {
            options.prefix.clone().bright_white().bold().to_string()
        },
        "target".bright_black().bold(),
        options.results.to_string().bright_white().bold(),
        "budget".bright_black().bold(),
        options.max_checks.to_string().bright_white().bold()
    );

    if let Some(path) = save_path {
        println!(
            "{} {}",
            "save".bright_black().bold(),
            path.display().to_string().bright_blue().bold()
        );
    }

    println!("{}", "-".repeat(72).bright_black());
}

fn update_progress(bar: &ProgressBar, progress: &SearchProgress, options: &SearchOptions) {
    bar.set_position(progress.checked.min(options.max_checks as u64));
    bar.set_message(format!(
        "hits {}/{} | batches {} | generated {}",
        progress.found, options.results, progress.batches, progress.generated
    ));
}

fn finish_progress(bar: &ProgressBar, summary: &SearchSummary) {
    let tone = summary_tone(&summary.stop_reason);
    bar.finish_and_clear();
    println!(
        "{} {}",
        "[done]".black().on_color(tone).bold(),
        summary.stop_reason.label().color(tone).bold()
    );
}

fn print_summary(summary: &SearchSummary) {
    let tone = summary_tone(&summary.stop_reason);
    println!(
        "{} {} | {} {} | {} {} | {} {}\n",
        "checked".bright_black().bold(),
        summary.progress.checked.to_string().color(tone).bold(),
        "found".bright_black().bold(),
        summary.progress.found.to_string().bright_green().bold(),
        "generated".bright_black().bold(),
        summary.progress.generated.to_string().bright_white().bold(),
        "batches".bright_black().bold(),
        summary.progress.batches.to_string().bright_white().bold()
    );
}

fn summary_tone(reason: &SearchStopReason) -> owo_colors::AnsiColors {
    match reason {
        SearchStopReason::ReachedResultTarget => owo_colors::AnsiColors::Green,
        SearchStopReason::ReachedCheckBudget => owo_colors::AnsiColors::Yellow,
        SearchStopReason::ExhaustedSearchSpace => owo_colors::AnsiColors::Blue,
        SearchStopReason::Cancelled => owo_colors::AnsiColors::Red,
    }
}
