use std::{iter::FusedIterator, rc::Rc};

use crate::expr::{BinopKind, Expr, ExprSkeleton, UnopKind, Variable};

use super::Synthesizer;

pub struct ExprIdx {
    arg_count: usize,
    limit_reached: bool,
    skele: Rc<ExprSkeleton>,
    hole_buff: Vec<usize>, // 0 -- const, n+1 -- argument n
}

impl ExprIdx {
    pub fn new(arg_count: usize) -> Self {
        Self {
            skele: Rc::new(Expr::Variable(())),
            limit_reached: true,
            arg_count,
            hole_buff: Vec::new(),
        }
    }

    pub fn reset(&mut self, new_skele: Rc<ExprSkeleton>) {
        self.hole_buff.clear();
        self.limit_reached = false;
        self.skele = new_skele;
        self.hole_buff.extend((0..self.skele.count_holes()).map(|_| 0));
    }

    fn digit_to_var(&self, digit: usize) -> Variable {
        if digit == 0 {
            Variable::Const
        } else {
            Variable::Argument(digit - 1)
        }
    }

    pub fn is_empty(&self) -> bool {
        self.limit_reached
    }

    pub fn increment(&mut self) {
        if self.limit_reached {
            return;
        }

        // NOTE: this assert failing is 100% a bug
        debug_assert!(self.hole_buff.iter().all(|x| *x <= self.arg_count));

        for digit in &mut self.hole_buff {
            if *digit < self.arg_count {
                *digit += 1;
                return;
            }

            *digit = 0;
        }

        self.limit_reached = true;
    }
}

impl Iterator for ExprIdx {
    type Item = Expr;

    fn next(&mut self) -> Option<Self::Item> {
        if self.limit_reached {
            return None;
        }

        let res = self.skele.to_expr(|idx|
            self.digit_to_var(self.hole_buff[idx])
        );

        self.increment();

        Some(res)
    }
}

impl FusedIterator for ExprIdx {

}

pub struct SkeletonIdx {
    depth_limit: usize,
    skeleton_idx: usize,
    skeletons: Vec<Rc<ExprSkeleton>>,
}

impl SkeletonIdx {
    pub fn new(depth_limit: usize) -> Self {
        Self {
            depth_limit,
            skeleton_idx: 0,
            skeletons: vec![Rc::new(Expr::Variable(()))],
        }
    }

    pub fn expand_holes(&mut self) {
        self.skeleton_idx = 0;

        let new_skeletons = self.skeletons.iter()
            .flat_map(|x| Self::grow_skeleton(&x))
            .map(Rc::new)
            .collect();

        self.skeletons = new_skeletons;

        self.skeletons.retain(|x| x.expr_depth() <= self.depth_limit);
    }

    // NOTE: this procedure is more than likely suboptimal
    fn grow_skeleton(skele: &'_ ExprSkeleton) -> impl Iterator<Item = ExprSkeleton> + '_ {
        let hole_count = skele.count_holes();

        (0..hole_count)
            .flat_map(move |hole_idx| Self::all_hole_substs()
                .map(move |subst| skele.subst_hole(hole_idx, &subst))
            )
    }

    fn all_hole_substs() -> impl Iterator<Item = ExprSkeleton> {
        // FIXME: it's better to use some crate for iterating over enum variants
        let all_unops = [
            UnopKind::Negate,
            UnopKind::Not,
        ];
        let all_binops = [
            BinopKind::And,
            BinopKind::Or,
            BinopKind::Xor,
            BinopKind::Plus,
            BinopKind::Minus,
            BinopKind::Shl,
            BinopKind::ShrA,
        ];

        let unop_substs = all_unops.into_iter()
            .map(|x| ExprSkeleton::Unop(
                x,
                Rc::new(ExprSkeleton::Variable(()))
            ));
        let binop_substs = all_binops.into_iter()
            .map(|x| ExprSkeleton::Binop(
                x,
                Rc::new(ExprSkeleton::Variable(())),
                Rc::new(ExprSkeleton::Variable(())),
            ));

        unop_substs.chain(binop_substs)
    }
}

impl Iterator for SkeletonIdx {
    type Item = Rc<ExprSkeleton>;

    fn next(&mut self) -> Option<Self::Item> {
        let res = self.skeletons.get(self.skeleton_idx)?.clone();

        self.skeleton_idx += 1;

        Some(res)
    }
}

impl FusedIterator for SkeletonIdx {

}

pub struct ExprBreadth {
    expr_iter: ExprIdx,
    skele_iter: SkeletonIdx,
}

impl ExprBreadth {
    pub fn new(arg_count: usize, depth_limit: usize) -> Self {
        Self {
            expr_iter: ExprIdx::new(arg_count),
            skele_iter: SkeletonIdx::new(depth_limit),
        }
    }

    pub fn next(&mut self) -> Option<Expr> {
        if self.expr_iter.is_empty() {
            let skele = self.skele_iter.next()
                .or_else(|| {
                    self.skele_iter.expand_holes();
                    self.skele_iter.next()
                });

            self.expr_iter.reset(skele?);
        }

        self.expr_iter.next()
    }
}

pub struct BruteEnum {
    breadth: ExprBreadth,
}

impl Synthesizer for BruteEnum {
    fn build(var_count: usize, depth_limit: usize) -> Self {
        Self {
            breadth: ExprBreadth::new(var_count, depth_limit),
        }
    }

    fn bad_cand(&mut self, _cand: &Expr) {
        // A brute doesn't learn
    }

    fn next_expr(&mut self) -> Option<Expr> {
        self.breadth.next()
    }
}