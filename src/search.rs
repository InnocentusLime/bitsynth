
use log::{debug, info};

use crate::{expr::{Expr, Value}, oracle::Oracle, synth::Synthesizer};
use crate::conv::*;

/// The report of the search routine
#[derive(Clone, Debug)]
pub enum SearchStep {
    /// The synthesizer has provided a sample `cand`
    /// that doesn't meet the specification.
    IncorrectSample {
        cand: Expr,
        /// This flag is set to `true` if a counterexample was found
        is_universally_wrong: bool,
    },
    /// The synthesizer has provided a sample `cand`
    /// that met the specification.
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
    /// Constructs the bithack searcher, parametrised by the
    /// synthesizer.
    ///
    /// If `should_learn` is true -- the searcher will try to look for
    /// a counterexample for invalid candidates.
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

    /// Forward an SMTLIB prompt to the verification oracle
    pub fn parse_prompt(
        &mut self,
        prompt: &str,
    ) {
        let preamble = self.converter.declaration();
        self.oracle.parse([preamble.as_str(), prompt].join("\n"));
    }

    pub fn converter(&self) -> &Z3ToExpr<'ctx> {
        &self.converter
    }

    pub fn oracle(&mut self) -> &mut Oracle<'ctx> {
        &mut self.oracle
    }

    /// Take a search step. `None` means that the search has terminated.
    /// For more information see [SearchStep].
    pub fn step(&mut self) -> Option<SearchStep> {
        let cand = self.synth.next_expr()?;
        let z3_cand = self.converter.expr_to_z3(&cand);

        debug!("Try: {cand:?}");

        Some(match self.oracle.check_candidate(&z3_cand, self.converter.z3_args()) {
            Some(model) => SearchStep::CorrectSample {
                answer: self.converter.build_answer(&cand, &model),
                cand,
            },
            None if !self.should_learn => {
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
                    info!("Counter-example: {args:?} -> {val}");
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