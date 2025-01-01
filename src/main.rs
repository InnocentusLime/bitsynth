use log::info;
use search::BithackSearch;
use synth::simple_search::SimpleSearch;
use z3::ast::Ast;

mod expr;
mod synth;
mod search;

use clap::{Parser, Subcommand};

#[derive(Parser)]
struct Cli {
    #[arg(long)]
    print_debug: bool,
}

fn main() {
    let cli = Cli::parse();

    if cli.print_debug {
        colog::default_builder()
            .filter_level(log::LevelFilter::Debug)
            .init();
    } else {
        colog::default_builder()
            .init();
    }

    let cfg = z3::Config::default();
    let ctx = z3::Context::new(&cfg);

    let mut search = BithackSearch::new(
        &ctx,
        SimpleSearch::new(),
        vec!["x".to_string()],
    );

    let x_var = search.get_argument("x").unwrap();
    let r_var = search.get_result_var();
    // search.add_constraint(
    //     r_var._eq(
    //         &z3::ast::BV::from_i64(&ctx, 0, 32)
    //     )
    // );
    search.add_constraint(
        r_var._eq(
           &(x_var & 0x2i64)
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
            } => info!("Explored: {cand:?} answer: {answer:?}"),
        }
    }
}