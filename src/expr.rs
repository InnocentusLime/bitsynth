use std::collections::{HashMap};

use bitvec::BitArr;

pub const BITS_PER_VAL: u32 = 32;
pub type ExprVal = i32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Variable {
    Const(usize),
    Argument(usize),
}

impl Variable {
    pub fn to_z3<'ctx>(&self, ctx: &'ctx z3::Context) -> z3::ast::BV<'ctx> {
        match self {
            Variable::Const(c) => z3::ast::BV::new_const(
                ctx,
                format!("c{c:}"),
                BITS_PER_VAL,
            ),
            Variable::Argument(x) => z3::ast::BV::new_const(
                ctx,
                format!("arg{x:}"),
                BITS_PER_VAL,
            ),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnopKind {
    Not,
    Negate,
    Shl(Variable),
    Shr(Variable),
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
    Binop(BinopKind, Box<(Expr, Expr)>),
}

impl Expr {
    pub fn compute<F>(
        &self,
        mut f: F,
    ) -> ExprVal
    where
        F: FnMut(Variable) -> ExprVal,
    {
        match self {
            Expr::Variable(v) => f(*v),
            Expr::Unop(unop_kind, e) => {
                let e = e.compute(&mut f);
                match unop_kind {
                    UnopKind::Not => !e,
                    UnopKind::Negate => -e,
                    UnopKind::Shl(n) => e << f(*n),
                    UnopKind::Shr(n) => e >> f(*n),
                }
            },
            Expr::Binop(binop_kind, lr) => {
                let (l, r) = (lr.0.compute(&mut f), lr.1.compute(&mut f));
                match binop_kind {
                    BinopKind::And => l & r,
                    BinopKind::Or => l | r,
                    BinopKind::Xor => l ^ r,
                    BinopKind::Plus => l + r,
                    BinopKind::Minus => l - r,
                }
            },
        }
    }

    pub fn to_z3<'ctx>(
        &self,
        ctx: &'ctx z3::Context,
        vars: &mut HashMap<Variable, z3::ast::BV<'ctx>>
    ) -> z3::ast::BV<'ctx> {
        let get_var = |vars: &mut HashMap<_, _>, v: &Variable| {
            vars.entry(*v)
                .or_insert_with(|| {
                    v.to_z3(ctx)
                })
                .clone()
        };

        match self {
            Expr::Variable(v) => get_var(vars, v),
            Expr::Unop(unop_kind, e) => {
                let e = e.to_z3(ctx, vars);
                match unop_kind {
                    UnopKind::Not => e.bvnot(),
                    UnopKind::Negate => e.bvneg(),
                    UnopKind::Shl(n) => e.bvshl(&get_var(vars, n)),
                    UnopKind::Shr(n) => e.bvashr(&get_var(vars, n)),
                }
            },
            Expr::Binop(binop_kind, lr) => {
                let (l, r) = (lr.0.to_z3(ctx, vars), lr.1.to_z3(ctx, vars));
                match binop_kind {
                    BinopKind::And => l.bvand(&r),
                    BinopKind::Or => l.bvor(&r),
                    BinopKind::Xor => l.bvxor(&r),
                    BinopKind::Plus => l.bvadd(&r),
                    BinopKind::Minus => l.bvsub(&r),
                }
            },
        }
    }
}