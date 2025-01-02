use log::debug;
use z3::ast::Ast;

use crate::expr::BITS_PER_VAL;

pub struct Oracle<'ctx> {
    z3: &'ctx z3::Context,
    solver: z3::Solver<'ctx>,
    result_var: z3::ast::BV<'ctx>,
    constraints: Vec<z3::ast::Bool<'ctx>>,
}

impl<'ctx> Oracle<'ctx> {
    pub fn new(
        z3: &'ctx z3::Context
    ) -> Self {
        Self {
            z3,
            constraints: Vec::new(),
            solver: z3::Solver::new(z3),
            result_var: z3::ast::BV::new_const(
                &z3,
                "res",
                BITS_PER_VAL,
            ),
        }
    }

    pub fn result_var(&self) -> &z3::ast::BV<'ctx> {
        &self.result_var
    }

    pub fn add_constraint(&mut self, constraint: z3::ast::Bool<'ctx>) {
        self.constraints.push(constraint);
    }

    pub fn has_universal_counterexample<'a>(
        &'a self,
        z3_cand: &'a z3::ast::BV<'ctx>,
        z3_consts: impl IntoIterator<Item = &'a z3::ast::BV<'ctx>>,
    ) -> bool
    where
        'ctx: 'a,
    {
        debug!("Searching for universal counter-example");

        let specif = self.counter_specif(&z3_cand, z3_consts);

        self.solver.push();
        self.solver.assert(&specif);
        let z3_verdict = self.solver.check();
        self.solver.pop(1);

        debug!("Z3 counterexample search: {z3_verdict:?}");

        z3_verdict == z3::SatResult::Sat
    }

    pub fn check_candidate<'a>(
        &'a self,
        z3_cand: &'a z3::ast::BV<'ctx>,
        z3_args: impl IntoIterator<Item = &'a z3::ast::BV<'ctx>>,
    ) -> Option<z3::Model<'ctx>>
    where
        'ctx: 'a,
    {
        debug!("Checking the candidate");

        let mut answer = None;

        self.solver.push();
        let specif = self.candidate_specif(&z3_cand, z3_args);
        self.solver.assert(&specif);
        let z3_verdict = self.solver.check();

        debug!("Z3 verdict: {z3_verdict:?}");

        if z3_verdict == z3::SatResult::Sat {
            answer = Some(self.solver.get_model().expect("Model must exist"));
        }

        self.solver.pop(1);

        answer
    }

    fn counter_specif<'a>(
        &'a self,
        cand: &z3::ast::BV<'ctx>,
        z3_consts: impl IntoIterator<Item = &'a z3::ast::BV<'ctx>>,
    ) -> z3::ast::Bool<'ctx> {
        let cand_constr = self.counter_constraint(cand);

        z3::ast::forall_const(
            &self.z3,
            &z3_consts.into_iter()
                .chain(std::iter::once(&self.result_var))
                .map(|x| x as &dyn z3::ast::Ast)
                .collect::<Vec<_>>()
            ,
            &[],
            &cand_constr,
        )
    }

    fn candidate_specif<'a>(
        &'a self,
        cand: &z3::ast::BV<'ctx>,
        z3_args: impl IntoIterator<Item = &'a z3::ast::BV<'ctx>>,
    ) -> z3::ast::Bool<'ctx>
    where
        'ctx: 'a
    {
        let cand_constr = self.cand_constraint(cand);

        z3::ast::forall_const(
            &self.z3,
            &z3_args.into_iter()
                .chain(std::iter::once(&self.result_var))
                .map(|x| x as &dyn z3::ast::Ast)
                .collect::<Vec<_>>()
            ,
            &[],
            &cand_constr,
        )
    }

    fn counter_constraint(&self, cand: &z3::ast::BV<'ctx>) -> z3::ast::Bool<'ctx> {
        let candeq = cand._eq(self.result_var());
        let specif = z3::ast::Bool::and(&self.z3,
            self.constraints.iter().collect::<Vec<_>>().as_slice()
        );

        candeq.implies(&!specif)
    }

    fn cand_constraint(&self, cand: &z3::ast::BV<'ctx>) -> z3::ast::Bool<'ctx> {
        let candeq = cand._eq(self.result_var());
        let specif = z3::ast::Bool::and(&self.z3,
            self.constraints.iter().collect::<Vec<_>>().as_slice()
        );

        candeq.implies(&specif)
    }
}