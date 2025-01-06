use expr::AnswerExpr;
use log::info;
use search::BithackSearch;
use synth::{brute_enum::BruteEnum, circuit_enum::CircuitEnum, simple_search::SimpleSearch};

mod search;
mod synth;
mod conv;
mod expr;
mod oracle;

use clap::Parser;

const BITSYNTH_STEP_LIMIMT: u64 = 10_000;

#[derive(Parser)]
struct Cli {
    #[arg(short, long)]
    trace: bool,
    #[arg(short, long)]
    verbose: bool,
    #[arg(long)]
    timeout: Option<u64>,
    #[arg(short, long)]
    constraint: Vec<String>,
    #[arg(short, long)]
    arg: Vec<String>,
}

fn perform_search(
    timeout: Option<u64>,
    constraint: Vec<String>,
    arg: Vec<String>,
) -> Option<AnswerExpr> {
    info!("Arguments: {:?}", arg);
    info!("Constraints: {:?}", constraint);

    let mut cfg = z3::Config::default();

    if let Some(timeout) = timeout {
        cfg.set_timeout_msec(timeout);
    }
    let ctx = z3::Context::new(&cfg);

    let mut search = BithackSearch::<CircuitEnum>::new(
        true,
        &ctx,
        arg,
        3,
    );

    search.parse_prompt(&constraint.join("\n"));

    let mut total_explored = 0;
    while let Some(step) = search.step() {
        total_explored += 1;

        if total_explored % 100 == 0 {
            println!("Explored: {total_explored}");
        }

        if total_explored == BITSYNTH_STEP_LIMIMT {
            println!("Too much");
            break;
        }

        match step {
            search::SearchStep::IncorrectSample {
                cand,
                is_universally_wrong,
            } => info!("Explored: {cand:?} bad: {is_universally_wrong}"),
            search::SearchStep::CorrectSample {
                cand,
                answer,
            } => {
                info!("Explored: {cand:?} answer: {answer:?}");
                println!("Total explored: {total_explored}");
                return Some(answer);
            },
        }
    }

    None
}

fn main() {
    let cli = Cli::parse();

    if cli.verbose {
        colog::default_builder()
            .filter_level(log::LevelFilter::Debug)
            .init();
    } else if cli.trace {
        colog::default_builder()
            .init();
    }

    match perform_search(cli.timeout, cli.constraint, cli.arg) {
        Some(ans) => println!("Found: {ans:}"),
        None => println!("No fitting expression found"),
    }
}