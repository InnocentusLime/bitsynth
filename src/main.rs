use expr::{AnswerExpr, BITS_PER_VAL};
use log::info;
use search::BithackSearch;
use synth::{brute_enum::BruteEnum, circuit_enum::CircuitEnum, simple_search::SimpleSearch};
use z3::ast::Ast;

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
}

fn perform_search(timeout: Option<u64>) -> Option<AnswerExpr> {
    let mut cfg = z3::Config::default();

    if let Some(timeout) = timeout {
        cfg.set_timeout_msec(timeout);
    }
    let ctx = z3::Context::new(&cfg);

    let mut search = BithackSearch::<CircuitEnum>::new(
        true,
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
    // search.oracle().add_constraint(
    //     r_var._eq(
    //        &(x_var * 8i64)
    //     )
    // );
    search.oracle().add_constraint(
        x_var.clone().bvsle(
            &z3::ast::BV::from_i64(&ctx, 0, BITS_PER_VAL)
        ).implies(
            &r_var._eq(&-x_var.clone())
        )
    );
    search.oracle().add_constraint(
        x_var.clone().bvsgt(
            &z3::ast::BV::from_i64(&ctx, 0, BITS_PER_VAL)
        ).implies(
            &r_var._eq(&x_var.clone())
        )
    );

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

    match perform_search(cli.timeout) {
        Some(ans) => println!("Found: {ans:}"),
        None => println!("No fitting expression found"),
    }
}