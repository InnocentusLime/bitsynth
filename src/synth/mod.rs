pub mod simple_search;

use std::collections::HashMap;

use crate::expr::{Expr, ExprVal, Variable};

pub struct Example {
    input: HashMap<Variable, ExprVal>,
    output: ExprVal,
}

/// A synthesizer is an iterator-like structure. It can generate
/// new expression candidates, but in addition to that it can also
/// be provided with examples to "learn". This allows synthesizers
/// to speed up the search.
pub trait Synthesizer {
    /// Supply the argument names to the synthesizer. The synthesizer
    /// cannot introduce new arguments to a formula, so it is required
    /// to use what it has.
    ///
    /// For simplicity, the variables are contigiously enumerated.
    fn known_args(&mut self, var_count: usize);

    /// Asks the synthesizer to take a new example into account. This is
    /// for potential optimisation of the search, so the synthesizer does not
    /// end up producing expressions, that "definitely aren't going to work".
    fn learn(&mut self, example: Example);

    /// Query the synthesizer for a next expression to try. The synthesizer
    /// may return `None` if it can no longer provide any new candidate.
    fn next_expr(&mut self) -> Option<Expr>;
}