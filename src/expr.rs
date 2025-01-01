pub const BITS_PER_VAL: u32 = 32;
pub type ExprVal = i32;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Value {
    Arg(String),
    Const(ExprVal),
}

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
pub enum Expr<V = Variable> {
    Variable(V),
    Unop(UnopKind, Box<Expr<V>>),
    Shift {
        is_left: bool,
        n: V,
        expr: Box<Expr<V>>,
    },
    Binop(BinopKind, Box<(Expr<V>, Expr<V>)>),
}

impl Expr {
    // NOTE: if stack starts overfilling -- that should be the first
    // function we turn non-recursive.
    pub fn walk_expr<T, V, U, S, B, Var, Prom>(
        &self,
        var_action: &mut V,
        unop_action: &mut U,
        shift_action: &mut S,
        binop_action: &mut B,
        var_promote: &mut Prom,
    ) -> T
    where
        V: FnMut(Variable) -> Var,
        U: FnMut(UnopKind, T) -> T,
        S: FnMut(bool, T, Var) -> T,
        B: FnMut(BinopKind, T, T) -> T,
        Prom: FnMut(Var) -> T,
    {
        match self {
            Expr::Variable(variable) => var_promote(var_action(*variable)),
            Expr::Unop(unop_kind, expr) => {
                let expr = expr.walk_expr(
                    var_action,
                    unop_action,
                    shift_action,
                    binop_action,
                    var_promote,
                );

                unop_action(*unop_kind, expr)
            },
            Expr::Shift { is_left, n, expr } => {
                let expr = expr.walk_expr(
                    var_action,
                    unop_action,
                    shift_action,
                    binop_action,
                    var_promote,
                );
                let n = var_action(*n);

                shift_action(*is_left, expr, n)
            },
            Expr::Binop(binop_kind, lr) => {
                let l = lr.0.walk_expr(
                    var_action,
                    unop_action,
                    shift_action,
                    binop_action,
                    var_promote,
                );

                let r = lr.1.walk_expr(
                    var_action,
                    unop_action,
                    shift_action,
                    binop_action,
                    var_promote,
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
            &mut |unop_kind, e: i32| match unop_kind {
                UnopKind::Not => -e,
                UnopKind::Negate => !e,
            },
            &mut |is_left, n, e| if is_left {
                e << n
            } else {
                e >> n
            },
            &mut |binop_kind, l, r| match binop_kind {
                BinopKind::And => l & r,
                BinopKind::Or => l | r,
                BinopKind::Xor => l ^ r,
                BinopKind::Plus => l + r,
                BinopKind::Minus => l - r,
            },
            &mut |x| x,
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
            &mut move |v| var_map(ctx, v),
            &mut |unop_kind, e: z3::ast::BV<'ctx>| match unop_kind {
                UnopKind::Not => !e,
                UnopKind::Negate => -e,
            },
            &mut |is_left, n, e| if is_left {
                e << n
            } else {
                e.bvashr(&n)
            },
            &mut |binop_kind, l, r| match binop_kind {
                BinopKind::And => l & r,
                BinopKind::Or => l | r,
                BinopKind::Xor => l ^ r,
                BinopKind::Plus => l + r,
                BinopKind::Minus => l - r,
            },
            &mut |x| x,
        )
    }

    pub fn to_ans<V>(
        &self,
        mut var_map: V,
    ) -> Expr<Value>
    where
        V: FnMut(Variable) -> Value,
    {
        self.walk_expr(
            &mut |v| var_map(v),
            &mut |unop_kind, e| {
                Expr::Unop(unop_kind, Box::new(e))
            },
            &mut |is_left, e, n| {
                Expr::Shift { is_left, n, expr: Box::new(e) }
            },
            &mut |binop_kind, l, r| {
                Expr::Binop(binop_kind, Box::new((l, r)))
            },
            &mut |x| Expr::Variable(x),
        )
    }
}