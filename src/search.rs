use std::collections::HashMap;

use crate::{expr::{Expr, Variable, BITS_PER_VAL}, synth::Synthesizer};

pub struct BithackSearch<'ctx, S> {
    solver: z3::Solver<'ctx>,
    z3: &'ctx z3::Context,
    synth: S,
    constraints: Vec<z3::ast::Bool<'ctx>>,
    result_var: z3::ast::BV<'ctx>,
    arguments: HashMap<String, usize>,
    // z3_args: Vec<z3::ast::BV<'ctx>>,
    // z3_consts: Vec<z3::ast::BV<'ctx>>,
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
        // let var_cache =
        //     arguments.values()
        //         .map(|x| *x)
        //         .map(Variable::Argument)
        //         .map(|x| (x, x.to_z3(z3)))
        //         .collect::<HashMap<_, _>>();

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
        }
    }

    pub fn get_result_var(&self) -> &z3::ast::BV<'ctx> {
        &self.result_var
    }

    pub fn get_argument(&self, x: &str) -> Option<&z3::ast::BV<'ctx>> {
        let id = self.arguments.get(x)?;

        // NOTE: yes, this may panic, but var_cache not having an entry
        // at this point would be a bug.
        // Some(&self.var_cache[&Variable::Argument(*id)])
        todo!()
    }

    /// Adds a new constraint to the searched expression. The constraint
    /// should involve the argument variables and the result variable.
    pub fn add_constraint(&mut self, constraint: z3::ast::Bool<'ctx>) {
        self.constraints.push(constraint);
    }
}