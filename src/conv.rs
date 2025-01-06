use std::collections::HashMap;

use log::trace;

use crate::expr::{AnswerExpr, Expr, ExprVal, Value, Variable, BITS_PER_VAL};

pub struct Z3ToExpr<'ctx> {
    z3: &'ctx z3::Context,
    arguments: HashMap<String, usize>,
    z3_consts: Vec<z3::ast::BV<'ctx>>,
    z3_args: Vec<z3::ast::BV<'ctx>>,
}

impl<'ctx> Z3ToExpr<'ctx> {
    pub fn new(
        z3: &'ctx z3::Context,
        arguments: impl IntoIterator<Item = String>,
    ) -> Self {
        let arguments =
            arguments.into_iter()
                .enumerate()
                .map(|(idx, arg_name)| (arg_name, idx))
                .collect::<HashMap<_, _>>();
        let z3_args = arguments.keys()
            .map(|name| Self::new_z3_arg(z3, name))
            .collect();

        Self {
            z3,
            arguments,
            z3_args,
            z3_consts: Vec::new(),
        }
    }

    pub fn z3_args(&self) -> &[z3::ast::BV<'ctx>] {
        &self.z3_args
    }

    pub fn z3_consts(&self) -> &[z3::ast::BV<'ctx>] {
        &self.z3_consts
    }

    pub fn declaration(&self) -> String {
        self.arguments.keys()
            .map(|x| x.as_str())
            .chain(std::iter::once("res"))
            .map(|x| format!("(declare-const {x:} (_ BitVec {BITS_PER_VAL:}))"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn get_argument(&self, x: &str) -> Option<&z3::ast::BV<'ctx>> {
        let id = self.arguments.get(x)?;

        // NOTE: yes, this may panic, but z3_args not having an entry
        // at this point would be a bug.
        Some(&self.z3_args[*id])
    }

    pub fn build_answer(&mut self, expr: &Expr, model: &z3::Model) -> AnswerExpr {
        let args = &self.arguments;
        let consts = &mut self.z3_consts;
        let mut next_const_idx = 0;

        expr.to_ans(
            |v| {
                match v {
                    Variable::Argument(idx) =>
                        args.iter()
                            .find(|(_, other)| **other == idx)
                            .map(|(name, _)| name.clone())
                            .map(Value::Arg)
                            .unwrap(),
                    Variable::UnknownConst => {
                        let c = &consts[next_const_idx];
                        let interp = model.get_const_interp(c).unwrap();
                        let val = interp.as_i64().unwrap() as i32;

                        next_const_idx += 1;

                        Value::Const(val)
                    },
                    Variable::Const(i) => {
                        Value::Const(i)
                    },
                }
            },
        )
    }

    pub fn build_counter_example(&self, model: &z3::Model) -> Vec<ExprVal> {
        self.z3_args.iter()
            .map(|x| model.get_const_interp(x).expect("No val for arg"))
            .map(|x| x.as_i64().unwrap() as ExprVal)
            .collect()
    }

    pub fn ans_expr_to_z3(&self, expr: &AnswerExpr) -> z3::ast::BV<'ctx> {
        expr.to_z3_ans(
            &self.z3,
            |v| self.get_argument(v).unwrap().clone()
        )
    }

    pub fn expr_to_z3(&mut self, expr: &Expr) -> z3::ast::BV<'ctx> {
        let args = &self.z3_args;
        let consts = &mut self.z3_consts;
        let mut next_const_idx = 0;

        trace!("Convert to z3: {expr:?}");

        expr.to_z3(
            &self.z3,
            |ctx, v| {
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
            |_ctx, idx| args[idx].clone(),
        )
    }

    fn new_z3_const(ctx: &z3::Context, idx: usize) -> z3::ast::BV<'_> {
        z3::ast::BV::new_const(
            ctx,
            format!("c{idx:}"),
            BITS_PER_VAL,
        )
    }

    fn new_z3_arg<'a>(ctx: &'a z3::Context, name: &str) -> z3::ast::BV<'a> {
        z3::ast::BV::new_const(
            ctx,
            name,
            BITS_PER_VAL,
        )
    }
}