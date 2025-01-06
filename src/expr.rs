use std::{fmt, rc::Rc};

pub const BITS_PER_VAL: u32 = 32;
pub type ExprVal = i32;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Value {
    Arg(String),
    Const(ExprVal),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Variable {
    UnknownConst,
    Const(ExprVal),
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
    Shl,
    ShrA,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Expr<V = Variable> {
    Variable(V),
    Unop(UnopKind, Rc<Expr<V>>),
    Binop(BinopKind, Rc<Expr<V>>, Rc<Expr<V>>),
}

pub type ExprSkeleton = Expr<()>;
pub type AnswerExpr = Expr<Value>;

impl<VarT> Expr<VarT> {
    // NOTE: if stack starts overfilling -- that should be the first
    // function we turn non-recursive.
    pub fn walk_expr<T, V, U, B, Var, Prom>(
        &self,
        var_action: &mut V,
        unop_action: &mut U,
        binop_action: &mut B,
        var_promote: &mut Prom,
    ) -> T
    where
        V: FnMut(&VarT) -> Var,
        U: FnMut(UnopKind, T) -> T,
        B: FnMut(BinopKind, T, T) -> T,
        Prom: FnMut(Var) -> T,
    {
        match self {
            Expr::Variable(variable) => var_promote(var_action(variable)),
            Expr::Unop(unop_kind, expr) => {
                let expr = expr.walk_expr(
                    var_action,
                    unop_action,
                    binop_action,
                    var_promote,
                );

                unop_action(*unop_kind, expr)
            },
            Expr::Binop(binop_kind, l, r) => {
                let l = l.walk_expr(
                    var_action,
                    unop_action,
                    binop_action,
                    var_promote,
                );

                let r = r.walk_expr(
                    var_action,
                    unop_action,
                    binop_action,
                    var_promote,
                );

                binop_action(*binop_kind, l, r)
            },
        }
    }

    pub fn expr_depth(&self) -> usize {
        self.walk_expr(
            &mut |_| 0,
            &mut |_, x| x + 1,
            &mut |_, l, r| 1 + std::cmp::max(l, r),
            &mut |x| x,
        )
    }
}

impl Expr {
    pub fn compute<F>(
        &self,
        mut var_map: F,
    ) -> ExprVal
    where
        F: FnMut(Variable) -> ExprVal,
    {
        self.walk_expr(
            &mut |x| var_map(*x),
            &mut |unop_kind, e: i32| match unop_kind {
                UnopKind::Not => -e,
                UnopKind::Negate => !e,
            },
            &mut |binop_kind, l, r| match binop_kind {
                BinopKind::And => l & r,
                BinopKind::Or => l | r,
                BinopKind::Xor => l ^ r,
                BinopKind::Plus => l + r,
                BinopKind::Minus => l - r,
                BinopKind::Shl => l << r,
                BinopKind::ShrA => l >> r,
            },
            &mut |x| x,
        )
    }

    pub fn to_z3<'ctx, A, C>(
        &self,
        ctx: &'ctx z3::Context,
        mut const_map: C,
        mut arg_map: A,
    ) -> z3::ast::BV<'ctx>
    where
        C: FnMut(&'ctx z3::Context, usize) -> z3::ast::BV<'ctx>,
        A: FnMut(&'ctx z3::Context, usize) -> z3::ast::BV<'ctx>,
    {
        let mut const_idx = 0;

        self.walk_expr(
            &mut move |v| match v {
                Variable::UnknownConst => {
                    let res = const_map(ctx, const_idx);
                    const_idx += 1;

                    res
                },
                Variable::Const(x) => z3::ast::BV::from_i64(ctx, *x as i64, BITS_PER_VAL),
                Variable::Argument(x) => arg_map(ctx, *x),
            },
            &mut |unop_kind, e: z3::ast::BV<'ctx>| match unop_kind {
                UnopKind::Not => !e,
                UnopKind::Negate => -e,
            },
            &mut |binop_kind, l, r| match binop_kind {
                BinopKind::And => l & r,
                BinopKind::Or => l | r,
                BinopKind::Xor => l ^ r,
                BinopKind::Plus => l + r,
                BinopKind::Minus => l - r,
                BinopKind::Shl => l << r,
                BinopKind::ShrA => l.bvashr(&r),
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
            &mut |v| var_map(*v),
            &mut |unop_kind, e| {
                Expr::Unop(unop_kind, Rc::new(e))
            },
            &mut |binop_kind, l, r| {
                Expr::Binop(binop_kind, Rc::new(l), Rc::new(r))
            },
            &mut |x| Expr::Variable(x),
        )
    }
}

impl AnswerExpr {
    pub fn to_z3_ans<'ctx, V>(
        &self,
        ctx: &'ctx z3::Context,
        mut var_map: V,
    ) -> z3::ast::BV<'ctx>
    where
        V: FnMut(&str) -> z3::ast::BV<'ctx>,
    {
        self.walk_expr(
            &mut move |v| match v {
                Value::Arg(x) => var_map(x.as_str()),
                Value::Const(x) => z3::ast::BV::from_i64(ctx, *x as i64, BITS_PER_VAL),
            },
            &mut |unop_kind, e: z3::ast::BV<'ctx>| match unop_kind {
                UnopKind::Not => !e,
                UnopKind::Negate => -e,
            },
            &mut |binop_kind, l, r| match binop_kind {
                BinopKind::And => l & r,
                BinopKind::Or => l | r,
                BinopKind::Xor => l ^ r,
                BinopKind::Plus => l + r,
                BinopKind::Minus => l - r,
                BinopKind::Shl => l << r,
                BinopKind::ShrA => l.bvashr(&r),
            },
            &mut |x| x,
        )
    }
}

impl fmt::Display for AnswerExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expr::Variable(x) => match x {
                Value::Arg(arg) => write!(f, "{arg}"),
                Value::Const(c) => write!(f, "{c}"),
            },
            Expr::Unop(unop_kind, expr) => match unop_kind {
                UnopKind::Not => write!(f, "!({expr})"),
                UnopKind::Negate => write!(f, "-({expr})"),
            },
            Expr::Binop(binop_kind, l, r) => match binop_kind {
                BinopKind::And => write!(f, "({l} & {r})"),
                BinopKind::Or => write!(f, "({l} | {r})"),
                BinopKind::Xor => write!(f, "({l} ^ {r})"),
                BinopKind::Plus => write!(f, "({l} + {r})"),
                BinopKind::Minus => write!(f, "({l} - {r})"),
                BinopKind::Shl => write!(f, "({l} << {r})"),
                BinopKind::ShrA => write!(f, "({l} >> {r})"),
            },
        }
    }
}

impl ExprSkeleton {
    pub fn morph<H, V, T, Prom>(
        &self,
        mut hole_action: H,
        mut promote: Prom,
    ) -> Expr<V>
    where
        H: FnMut(usize) -> T,
        Prom: FnMut(T) -> Expr<V>,
    {
        let mut hole_idx = 0;
        let mut hole_action = move |_: &()| {
            let res = hole_action(hole_idx);
            hole_idx += 1;

            res
        };

        self.walk_expr(
            &mut hole_action,
            &mut |unop_kind, e| {
                Expr::Unop(unop_kind, Rc::new(e))
            },
            &mut |binop_kind, l, r| {
                Expr::Binop(binop_kind, Rc::new(l), Rc::new(r))
            },
            &mut promote
        )
    }

    pub fn to_expr<V>(&self, hole_action: V) -> Expr
    where
        V: FnMut(usize) -> Variable,
    {
        self.morph(hole_action, Expr::Variable)
    }

    pub fn subst_hole(&self, target_idx: usize, skele: &ExprSkeleton) -> Self
    {
        self.morph(
            |hole_idx| if hole_idx == target_idx {
                skele.clone()
            } else {
                ExprSkeleton::Variable(())
            },
            |x| x
        )
    }

    pub fn count_holes(&self) -> usize {
        self.walk_expr(
            &mut |_| 1,
            &mut |_, e| e,
            &mut |_, l, r| l + r,
            &mut |x| x
        )
    }
}