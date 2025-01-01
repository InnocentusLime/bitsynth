use std::collections::HashMap;

use log::{debug, trace};
use z3::{ast::Ast, SatResult};

use crate::{expr::{Expr, Value, BITS_PER_VAL}, synth::Synthesizer};
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
    solver: z3::Solver<'ctx>,
    z3: &'ctx z3::Context,
    synth: S,
    constraints: Vec<z3::ast::Bool<'ctx>>,
    result_var: z3::ast::BV<'ctx>,
    converter: Z3ToExpr<'ctx>,
}

impl<'ctx, S: Synthesizer> BithackSearch<'ctx, S> {
    pub fn new(
        z3: &'ctx z3::Context,
        mut synth: S,
        arguments: Vec<String>,
    ) -> Self {
        synth.known_args(arguments.len());

        Self {
            solver: z3::Solver::new(z3),
            z3,
            synth,
            constraints: vec![],
            converter: Z3ToExpr::new(z3, arguments),
            result_var: z3::ast::BV::new_const(
                &z3,
                "result",
                BITS_PER_VAL,
            ),
        }
    }

    pub fn converter(&self) -> &Z3ToExpr<'ctx> {
        &self.converter
    }

    pub fn get_result_var(&self) -> &z3::ast::BV<'ctx> {
        &self.result_var
    }

    /// Adds a new constraint to the searched expression. The constraint
    /// should involve the argument variables and the result variable.
    pub fn add_constraint(&mut self, constraint: z3::ast::Bool<'ctx>) {
        self.constraints.push(constraint);
    }

    pub fn step(&mut self) -> Option<SearchStep> {
        let cand = self.synth.next_expr()?;
        let z3_cand = self.converter.expr_to_z3(&cand);
        let mut answer = None;

        debug!("Try: {cand:?}");

        self.solver.push();
        let specif = self.candidate_specif(&z3_cand);
        self.solver.assert(&specif);
        let z3_verdict = self.solver.check();
        debug!("Z3 verdict: {z3_verdict:?}");
        let is_good = match z3_verdict {
            z3::SatResult::Unsat | z3::SatResult::Unknown => false,
            z3::SatResult::Sat => true,
        };

        if is_good {
            let model = self.solver.get_model().unwrap();
            answer = Some(self.converter.build_ans(&cand, &model))
        }

        self.solver.pop(1);

        match answer {
            None => {
                let specif = self.counter_specif(&z3_cand);

                self.solver.push();
                self.solver.assert(&specif);
                let z3_verdict = self.solver.check();
                self.solver.pop(1);

                debug!("Z3 counterexample search: {z3_verdict:?}");

                Some(SearchStep::IncorrectSample {
                    cand,
                    is_universally_wrong: z3_verdict == SatResult::Sat,
                })
            },
            Some(answer) => Some(SearchStep::CorrectSample {
                cand,
                answer,
            })
        }
    }

    fn counter_specif(&mut self, cand: &z3::ast::BV<'ctx>) -> z3::ast::Bool<'ctx> {
        let cand_constr = self.counter_constraint(cand);

        z3::ast::forall_const(
            &self.z3,
            &self.converter.z3_consts().iter()
                .chain(std::iter::once(&self.result_var))
                .map(|x| x as &dyn z3::ast::Ast)
                .collect::<Vec<_>>()
            ,
            &[],
            &cand_constr,
        )
    }

    fn candidate_specif(&mut self, cand: &z3::ast::BV<'ctx>) -> z3::ast::Bool<'ctx> {
        let cand_constr = self.cand_constraint(cand);

        z3::ast::forall_const(
            &self.z3,
            &self.converter.z3_args().iter()
                .chain(std::iter::once(&self.result_var))
                .map(|x| x as &dyn z3::ast::Ast)
                .collect::<Vec<_>>()
            ,
            &[],
            &cand_constr,
        )
    }

    fn counter_constraint(&mut self, cand: &z3::ast::BV<'ctx>) -> z3::ast::Bool<'ctx> {
        let candeq = cand._eq(self.get_result_var());
        let specif = z3::ast::Bool::and(&self.z3,
            self.constraints.iter().collect::<Vec<_>>().as_slice()
        );

        candeq.implies(&!specif)
    }

    fn cand_constraint(&mut self, cand: &z3::ast::BV<'ctx>) -> z3::ast::Bool<'ctx> {
        let candeq = cand._eq(self.get_result_var());
        let specif = z3::ast::Bool::and(&self.z3,
            self.constraints.iter().collect::<Vec<_>>().as_slice()
        );

        candeq.implies(&specif)
    }
}