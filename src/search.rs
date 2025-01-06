
use log::debug;

use crate::{expr::{Expr, Value}, oracle::Oracle, synth::Synthesizer};
use crate::conv::*;

#[derive(Clone, Debug)]
pub enum SearchStep {
    IncorrectSample {
        cand: Expr,
        is_universally_wrong: bool,
    },
    CorrectSample {
        cand: Expr,
        answer: Expr<Value>,
    },
}

pub struct BithackSearch<'ctx, S> {
    should_learn: bool,
    synth: S,
    oracle: Oracle<'ctx>,
    converter: Z3ToExpr<'ctx>,
}

impl<'ctx, S: Synthesizer<'ctx>> BithackSearch<'ctx, S> {
    pub fn new(
        should_learn: bool,
        z3: &'ctx z3::Context,
        arguments: Vec<String>,
        depth_limit: usize,
    ) -> Self {
        Self {
            should_learn,
            synth: S::build(z3, arguments.len(), depth_limit),
            converter: Z3ToExpr::new(z3, arguments),
            oracle: Oracle::new(z3),
        }
    }

    pub fn converter(&self) -> &Z3ToExpr<'ctx> {
        &self.converter
    }

    pub fn oracle(&mut self) -> &mut Oracle<'ctx> {
        &mut self.oracle
    }

    pub fn step(&mut self) -> Option<SearchStep> {
        let cand = self.synth.next_expr()?;
        let z3_cand = self.converter.expr_to_z3(&cand);

        debug!("Try: {cand:?}");

        Some(match self.oracle.check_candidate(&z3_cand, self.converter.z3_args()) {
            Some(model) => SearchStep::CorrectSample {
                answer: self.converter.build_answer(&cand, &model),
                cand,
            },
            None if self.should_learn => {
                SearchStep::IncorrectSample {
                    is_universally_wrong: false,
                    cand,
                }
            },
            None => {
                let counterexample = self.oracle.counterexample(
                    &z3_cand,
                    self.converter.z3_consts()
                );

                if let Some(model) = &counterexample {
                    let args = self.converter.build_counter_example(model);
                    let val = self.oracle.suitable_value(
                        self.converter.z3_args().iter(),
                        args.iter().map(|x| *x),
                    );
                    self.synth.bad_cand(&cand, args, val);
                }

                SearchStep::IncorrectSample {
                    is_universally_wrong: counterexample.is_some(),
                    cand,
                }
            },
        })
    }
}