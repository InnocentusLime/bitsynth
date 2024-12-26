use log::info;
use search::BithackSearch;
use synth::simple_search::SimpleSearch;
use z3::ast::Ast;

mod expr;
mod synth;
mod search;

fn main() {
    let logger = colog::default_builder()
        .filter_level(log::LevelFilter::Debug)
        .init();

    info!("Started");

    let cfg = z3::Config::default();
    let ctx = z3::Context::new(&cfg);

    let mut search = BithackSearch::new(
        &ctx,
        SimpleSearch::new(),
        vec!["x".to_string()],
    );

    // let x_var = search.get_argument("x").unwrap();
    let r_var = search.get_result_var();
    search.add_constraint(
        r_var._eq(
            &z3::ast::BV::from_i64(&ctx, 0, 32)
        )
    );

    while let Some((_cand, _is_good)) = search.step() {
        info!("Something")
    }

    info!("I am leaving");
}