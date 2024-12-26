use std::collections::HashMap;

use z3::ast::Ast;

use crate::{expr::{Expr, Variable, BITS_PER_VAL}, synth::Synthesizer};

pub struct BithackSearch<'ctx, S> {
    solver: z3::Solver<'ctx>,
    z3: &'ctx z3::Context,
    synth: S,
    constraints: Vec<z3::ast::Bool<'ctx>>,
    result_var: z3::ast::BV<'ctx>,
    arguments: HashMap<String, usize>,
    z3_consts: Vec<z3::ast::BV<'ctx>>,
    z3_args: Vec<z3::ast::BV<'ctx>>,
}

impl<'ctx, S: Synthesizer> BithackSearch<'ctx, S> {
    pub fn new(
        z3: &'ctx z3::Context,
        synth: S,
        arguments: Vec<String>,
    ) -> Self {
        let arguments =
            arguments.into_iter()
                .enumerate()
                .map(|(x, y)| (y, x))
                .collect::<HashMap<_, _>>();
        let z3_args = (0..arguments.len())
            .map(|idx| Self::new_z3_arg(z3, idx))
            .collect();

        Self {
            solver: z3::Solver::new(z3),
            z3,
            synth,
            constraints: vec![],
            result_var: z3::ast::BV::new_const(
                &z3,
                "result",
                BITS_PER_VAL,
            ),
            arguments,
            z3_consts: Vec::new(),
            z3_args,
        }
    }

    pub fn get_result_var(&self) -> &z3::ast::BV<'ctx> {
        &self.result_var
    }

    pub fn get_argument(&self, x: &str) -> Option<&z3::ast::BV<'ctx>> {
        let id = self.arguments.get(x)?;

        // NOTE: yes, this may panic, but z3_args not having an entry
        // at this point would be a bug.
        Some(&self.z3_args[*id])
    }

    /// Adds a new constraint to the searched expression. The constraint
    /// should involve the argument variables and the result variable.
    pub fn add_constraint(&mut self, constraint: z3::ast::Bool<'ctx>) {
        self.constraints.push(constraint);
    }

    fn next_cand(&mut self) -> Option<(Expr, bool)> {
        let cand = self.synth.next_expr()?;

        let z3_cand = self.expr_to_z3(&cand);
        let specif = self.candidate_specif(z3_cand);

        self.solver.assert(&specif);

        let is_good = match self.solver.check() {
            z3::SatResult::Unsat | z3::SatResult::Unknown => false,
            z3::SatResult::Sat => true,
        };

        Some((cand, is_good))
    }

    fn candidate_specif(&mut self, cand: z3::ast::BV<'ctx>) -> z3::ast::Bool<'ctx> {
        let cand_constr = self.cand_constraint(cand);

        z3::ast::forall_const(
            &self.z3,
            &self.z3_args.iter()
                .map(|x| x as &dyn z3::ast::Ast)
                .collect::<Vec<_>>()
            ,
            &[],
            &cand_constr,
        )
    }

    fn cand_constraint(&mut self, cand: z3::ast::BV<'ctx>) -> z3::ast::Bool<'ctx> {
        let candeq = cand._eq(self.get_result_var());

        self.constraints.iter()
            .fold(candeq, |acc, constraint| {
                acc & constraint
            })
    }

    fn expr_to_z3(&mut self, expr: &Expr) -> z3::ast::BV<'ctx> {
        let args = &self.z3_args;
        let consts = &mut self.z3_consts;
        let mut next_const_idx = 0;

        expr.to_z3(
            &self.z3,
            |ctx, v| match v {
                Variable::Argument(idx) => args[idx].clone(),
                Variable::Const => {
                    let res = match consts.get(next_const_idx) {
                        Some(x) => x.clone(),
                        None => {
                            let c = Self::new_z3_const(ctx, next_const_idx);
                            consts.push(c.clone());
                            c
                        },
                    };

                    next_const_idx += 1;

                    res
                },
            },
        )
    }

    fn new_z3_const(ctx: &z3::Context, idx: usize) -> z3::ast::BV<'_> {
        z3::ast::BV::new_const(
            ctx,
            format!("c{idx:}"),
            BITS_PER_VAL,
        )
    }

    fn new_z3_arg(ctx: &z3::Context, idx: usize) -> z3::ast::BV<'_> {
        z3::ast::BV::new_const(
            ctx,
            format!("arg{idx:}"),
            BITS_PER_VAL,
        )
    }
}