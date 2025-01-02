use bitsynth::synth::brute_enum::BruteEnum;
use bitsynth::synth::simple_search::SimpleSearch;
use bitsynth::{conv::Z3ToExpr, expr::BITS_PER_VAL, search::BithackSearch, synth::Synthesizer};
use z3::ast::Ast;

pub const EASY_DEPTH_LIMIT: usize = 5;
pub const EASY_SEARCH_LIMIT: usize = 100_000;

pub struct FuneqChallenge {
    args: Vec<String>,
    builder: Box<dyn for<'ctx> Fn(&'ctx z3::Context, &Z3ToExpr<'ctx>) -> z3::ast::BV<'ctx>>,
}

impl FuneqChallenge {
    pub fn perform_tests<'ctx, S: Synthesizer>(
        tests: impl IntoIterator<Item = FuneqChallenge>,
        z3: &'ctx z3::Context,
    ) {
        let solver = z3::Solver::new(z3);
        let tester = move |l: &z3::ast::BV, r: &z3::ast::BV| {
            solver.push();
            solver.assert(&!l._eq(&r));
            let veridct = solver.check();
            solver.pop(1);

            veridct == z3::SatResult::Unsat
        };

        tests.into_iter().for_each(|x| x.perform::<S, _>(z3, &tester));
    }

    fn perform<'ctx, S, O>(
        self,
        z3: &'ctx z3::Context,
        mut tester: O,
    )
    where
        S: Synthesizer,
        O: FnMut(&z3::ast::BV, &z3::ast::BV) -> bool,
    {
        let mut search = BithackSearch::<S>::new(
            z3,
            self.args.clone(),
            EASY_DEPTH_LIMIT,
        );
        let fun = (self.builder)(z3, search.converter());

        let res_var = search.oracle().result_var().clone();
        search.oracle().add_constraint(res_var._eq(&fun));

        let mut found = false;
        let mut step_cnt = 0;
        while let Some(step) = search.step() {
            if step_cnt >= EASY_SEARCH_LIMIT { break; }
            match step {
                // TODO: fact check the synthesizer there?
                bitsynth::search::SearchStep::IncorrectSample { .. } => (),
                bitsynth::search::SearchStep::CorrectSample {
                    answer,
                    ..
                } => {
                    let res = search.converter().ans_expr_to_z3(&answer);
                    assert!(tester(&fun, &res));
                    found = true;
                },
            }

            step_cnt += 1;
        }

        assert!(found)
    }
}

pub fn run_tests_with_z3<F>(f: F)
where
    F: FnOnce(z3::Context),
{
    let _ = colog::default_builder()
        .filter_level(log::LevelFilter::Debug)
        .try_init();

    let cfg = z3::Config::default();
    let ctx = z3::Context::new(&cfg);

    f(ctx)
}

pub fn simple_funeq_challenges() -> Vec<FuneqChallenge> {
    vec![
        FuneqChallenge {
            args: vec!["x".to_string()],
            builder: Box::new(|_z3, conv| {
                let x = conv.get_argument("x").unwrap().clone();

                x & 0x2i64
            }),
        },
        FuneqChallenge {
            args: vec!["x".to_string()],
            builder: Box::new(|_z3, conv| {
                let x = conv.get_argument("x").unwrap().clone();

                x
            }),
        },
        FuneqChallenge {
            args: vec!["x".to_string()],
            builder: Box::new(|z3, _conv| {
                z3::ast::BV::from_i64(z3, 123, BITS_PER_VAL)
            }),
        },
    ]
}

#[test]
fn test_simple_search_simple_funeq() {
    run_tests_with_z3(|z3| {
        let tests = simple_funeq_challenges();
        FuneqChallenge::perform_tests::<SimpleSearch>(tests, &z3);
    });
}

#[test]
fn test_brute_enum_search_simple_funeq() {
    run_tests_with_z3(|z3| {
        let tests = simple_funeq_challenges();
        FuneqChallenge::perform_tests::<BruteEnum>(tests, &z3);
    });
}