use crate::expr::Expr;

use super::Synthesizer;


/// This synthesizer simply tries a few
/// common examples.
pub struct SimpleSearch {
    db: Vec<Expr>,
    last_tried: usize,
}

impl Synthesizer for SimpleSearch {
    fn known_args(&mut self, vars: usize) {
        todo!()
    }

    fn learn(&mut self, example: super::Example) {
        todo!()
    }

    fn next_expr(&mut self) -> Option<Expr> {
        todo!()
    }
}