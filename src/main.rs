use expr::AnswerExpr;
use log::info;
use search::BithackSearch;
use synth::{brute_enum::BruteEnum, simple_search::SimpleSearch};
use z3::ast::Ast;

mod search;
mod synth;
mod conv;
mod expr;
mod oracle;

use clap::Parser;

#[derive(Parser)]
struct Cli {
    #[arg(short, long)]
    trace: bool,
    #[arg(short, long)]
    verbose: bool,
}

fn perform_search() -> Option<AnswerExpr> {
    let mut cfg = z3::Config::default();
    // cfg.set_timeout_msec(300);
    let ctx = z3::Context::new(&cfg);

    let mut search = BithackSearch::<BruteEnum>::new(
        false,
        &ctx,
        vec!["x".to_string()],
        3,
    );

    let r_var = search.oracle().result_var().clone();
    let x_var = search.converter().get_argument("x").unwrap().clone();
    // search.add_constraint(
    //     r_var._eq(
    //         &z3::ast::BV::from_i64(&ctx, 0, 32)
    //     )
    // );
    search.oracle().add_constraint(
        r_var._eq(
           &(x_var * 8i64)
        )
    );

    while let Some(step) = search.step() {
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

    match perform_search() {
        Some(ans) => println!("Found: {ans:}"),
        None => println!("No fitting expression found"),
    }
}