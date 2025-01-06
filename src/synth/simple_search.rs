use std::rc::Rc;

use crate::expr::{Expr, ExprVal, Variable};

use super::Synthesizer;


/// This synthesizer simply tries a few
/// common examples. Works only for single-var
/// functions.
pub struct SimpleSearch {
    arg_cnt: usize,
    db: Vec<Expr>,
    last_tried: usize,
}

impl SimpleSearch {
    pub fn new(arg_cnt: usize) -> Self {
        SimpleSearch {
            arg_cnt,
            last_tried: 0,
            db: vec![
                Expr::Variable(Variable::UnknownConst),
                Expr::Variable(Variable::Argument(0)),
                Expr::Binop(
                    crate::expr::BinopKind::And,
                    Rc::new(Expr::Variable(Variable::Argument(0))),
                    Rc::new(Expr::Variable(Variable::UnknownConst)),
                )
            ],
         }
    }
}

impl<'ctx> Synthesizer<'ctx> for SimpleSearch {
    fn build(_z3: &'ctx z3::Context, var_count: usize, _depth_limit: usize) -> Self {
        Self::new(var_count)
    }

    fn bad_cand(&mut self, _expr: &Expr, _args: Vec<ExprVal>, _val: ExprVal) {
        /* We do not learn. */
    }

    fn next_expr(&mut self) -> Option<Expr> {
        if self.arg_cnt < 1 {
            return None;
        }

        match self.db.get(self.last_tried) {
            None => None,
            Some(x) => {
                self.last_tried += 1;
                Some(x.clone())
            },
        }
    }
}