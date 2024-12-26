pub const BITS_PER_VAL: u32 = 32;
pub type ExprVal = i32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Variable {
    Const,
    Argument(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnopKind {
    Not,
    Negate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinopKind {
    And,
    Or,
    Xor,
    Plus,
    Minus,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Expr {
    Variable(Variable),
    Unop(UnopKind, Box<Expr>),
    Shift {
        is_left: bool,
        n: Variable,
        expr: Box<Expr>,
    },
    Binop(BinopKind, Box<(Expr, Expr)>),
}

impl Expr {
    // NOTE: if stack starts overfilling -- that should be the first
    // function we turn non-recursive.
    pub fn walk_expr<T, V, U, S, B>(
        &self,
        mut var_action: V,
        mut unop_action: U,
        mut shift_action: S,
        mut binop_action: B,
    ) -> T
    where
        V: FnMut(Variable) -> T,
        U: FnMut(UnopKind, T) -> T,
        S: FnMut(bool, T, T) -> T,
        B: FnMut(BinopKind, T, T) -> T,
    {
        match self {
            Expr::Variable(variable) => var_action(*variable),
            Expr::Unop(unop_kind, expr) => {
                let expr = expr.walk_expr(
                    &mut var_action,
                    &mut unop_action,
                    &mut shift_action,
                    &mut binop_action,
                );

                unop_action(*unop_kind, expr)
            },
            Expr::Shift { is_left, n, expr } => {
                let expr = expr.walk_expr(
                    &mut var_action,
                    &mut unop_action,
                    &mut shift_action,
                    &mut binop_action,
                );
                let n = var_action(*n);

                shift_action(*is_left, expr, n)
            },
            Expr::Binop(binop_kind, lr) => {
                let l = lr.0.walk_expr(
                    &mut var_action,
                    &mut unop_action,
                    &mut shift_action,
                    &mut binop_action
                );

                let r = lr.0.walk_expr(
                    &mut var_action,
                    &mut unop_action,
                    &mut shift_action,
                    &mut binop_action
                );

                binop_action(*binop_kind, l, r)
            },
        }
    }

    pub fn compute<F>(
        &self,
        mut var_map: F,
    ) -> ExprVal
    where
        F: FnMut(Variable) -> ExprVal,
    {
        self.walk_expr(
            &mut var_map,
            |unop_kind, e| match unop_kind {
                UnopKind::Not => -e,
                UnopKind::Negate => !e,
            },
            |is_left, n, e| if is_left {
                e << n
            } else {
                e >> n
            },
            |binop_kind, l, r| match binop_kind {
                BinopKind::And => l & r,
                BinopKind::Or => l | r,
                BinopKind::Xor => l ^ r,
                BinopKind::Plus => l + r,
                BinopKind::Minus => l - r,
            },
        )
    }

    pub fn to_z3<'ctx, V>(
        &self,
        ctx: &'ctx z3::Context,
        mut var_map: V,
    ) -> z3::ast::BV<'ctx>
    where
        V: FnMut(&'ctx z3::Context, Variable) -> z3::ast::BV<'ctx>,
    {
        self.walk_expr(
            move |v| var_map(ctx, v),
            |unop_kind, e| match unop_kind {
                UnopKind::Not => !e,
                UnopKind::Negate => -e,
            },
            |is_left, n, e| if is_left {
                e << n
            } else {
                e.bvashr(&n)
            },
            |binop_kind, l, r| match binop_kind {
                BinopKind::And => l & r,
                BinopKind::Or => l | r,
                BinopKind::Xor => l ^ r,
                BinopKind::Plus => l + r,
                BinopKind::Minus => l - r,
            },
        )
    }
}