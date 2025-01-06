pub mod simple_search;
pub mod brute_enum;
pub mod circuit_enum;

use crate::expr::{Expr, ExprVal};

/// A synthesizer is an iterator-like structure. It can generate
/// new expression candidates, but in addition to that it can also
/// be provided with examples to "learn". This allows synthesizers
/// to speed up the search.
pub trait Synthesizer {
    fn build(var_count: usize, depth_limit: usize) -> Self;

    /// Reports to the synthesizer, that the produced candidate is
    /// "universally bad". This can be used to reduce the search space.
    fn bad_cand(&mut self, cand: &Expr, args: Vec<ExprVal>, expected: ExprVal);

    /// Query the synthesizer for a next expression to try. The synthesizer
    /// may return `None` if it can no longer provide any new candidate.
    fn next_expr(&mut self) -> Option<Expr>;
}