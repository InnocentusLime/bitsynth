use std::rc::Rc;

use crate::expr::{Expr, Variable};

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
                Expr::Variable(Variable::Const),
                Expr::Variable(Variable::Argument(0)),
                Expr::Binop(
                    crate::expr::BinopKind::And,
                    Rc::new(Expr::Variable(Variable::Argument(0))),
                    Rc::new(Expr::Variable(Variable::Const)),
                )
            ],
         }
    }
}

impl Synthesizer for SimpleSearch {
    fn build(var_count: usize, _depth_limit: usize) -> Self {
        Self::new(var_count)
    }

    fn learn(&mut self, _example: super::Example) {
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