use log::{debug, info};
use z3::ast::Ast;

use crate::expr::{ExprVal, BITS_PER_VAL};

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

    pub fn parse(&mut self, str: String) {
        // This is SHIT. Don't do this kids!
        info!("Going to parse: {str}");

        self.solver.reset();
        self.solver.from_string(str.as_bytes());
        self.constraints = self.solver.get_assertions().into_iter()
            .map(|x| unsafe {
                z3::ast::Bool::wrap(
                    &self.z3,
                    x.get_z3_ast()
                )
            })
            .collect();
        self.solver.reset();

        info!("Input constraints: {:?}", self.constraints);
    }

    pub fn counterexample<'a>(
        &'a self,
        z3_cand: &'a z3::ast::BV<'ctx>,
        z3_consts: impl IntoIterator<Item = &'a z3::ast::BV<'ctx>>,
    ) -> Option<z3::Model<'ctx>>
    where
        'ctx: 'a,
    {
        let mut answer = None;
        debug!("Searching for universal counter-example");

        let specif = self.counter_specif(&z3_cand, z3_consts);

        self.solver.push();
        self.solver.assert(&specif);
        let z3_verdict = self.solver.check();

        debug!("Z3 counterexample search: {z3_verdict:?}");

        if z3_verdict == z3::SatResult::Sat {
            answer = Some(self.solver.get_model().expect("Model must exist"));
        }

        self.solver.pop(1);

        answer
    }

    pub fn suitable_value<'a>(
        &self,
        z3_args: impl IntoIterator<Item = &'a z3::ast::BV<'ctx>>,
        z3_arg_values: impl IntoIterator<Item = ExprVal>,
    ) -> ExprVal
    where
        'ctx: 'a,
    {
        debug!("Generating a valid value");

        self.solver.push();

        self.solver.assert(&z3::ast::Bool::and(&self.z3,
            self.constraints.iter().collect::<Vec<_>>().as_slice()
        ));
        for (arg, val) in z3_args.into_iter().zip(z3_arg_values.into_iter()) {
            self.solver.assert(&arg._eq(
                &z3::ast::BV::from_i64(&self.z3, val as i64, BITS_PER_VAL)
            ));
        }

        assert!(self.solver.check() == z3::SatResult::Sat);

        let ans = self.solver.get_model()
            .unwrap()
            .get_const_interp(&self.result_var)
            .unwrap()
            .as_i64()
            .unwrap() as ExprVal;

        self.solver.pop(1);

        ans
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