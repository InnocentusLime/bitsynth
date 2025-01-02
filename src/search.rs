
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

impl<'ctx, S: Synthesizer> BithackSearch<'ctx, S> {
    pub fn new(
        should_learn: bool,
        z3: &'ctx z3::Context,
        arguments: Vec<String>,
        depth_limit: usize,
    ) -> Self {
        Self {
            should_learn,
            synth: S::build(arguments.len(), depth_limit),
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
            None => {
                let is_universally_wrong = if self.should_learn {
                    self.oracle.has_universal_counterexample(
                        &z3_cand,
                        self.converter.z3_consts(),
                    )
                } else {
                    false
                };

                if is_universally_wrong {
                    self.synth.bad_cand(&cand);
                }

                SearchStep::IncorrectSample {
                    is_universally_wrong,
                    cand,
                }
            },
        })
    }
}