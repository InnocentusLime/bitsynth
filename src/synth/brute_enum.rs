use crate::expr::{BinopKind, Expr, ExprSkeleton, UnopKind, Variable};

use super::Synthesizer;

pub const DEFAULT_BREADTH_LIMIT: usize = 1_000;

pub struct ExprIdx {
    arg_count: usize,
    limit_reached: bool,
    hole_buff: Vec<usize>, // 0 -- const, n+1 -- argument n
}

impl ExprIdx {
    pub fn new(arg_count: usize) -> Self {
        Self {
            limit_reached: false,
            arg_count,
            hole_buff: Vec::new(),
        }
    }

    pub fn reset(&mut self, skele: &ExprSkeleton) {
        self.hole_buff.clear();
        self.limit_reached = false;
        self.hole_buff.extend((0..skele.count_holes()).map(|_| 0));
    }

    pub fn next_expr(&mut self, skele: &ExprSkeleton) -> Option<Expr> {
        if self.limit_reached { return None; }

        let res = self.produce(skele);
        self.increment();

        Some(res)
    }

    pub fn produce(&self, skele: &ExprSkeleton) -> Expr {
        // NOTE: this assert failing is 100% an API misuse
        assert_eq!(self.hole_buff.len(), skele.count_holes(), "You forgot to call reset");

        skele.to_expr(|idx| self.digit_to_var(self.hole_buff[idx]))
    }

    fn digit_to_var(&self, digit: usize) -> Variable {
        if digit == 0 {
            Variable::Const
        } else {
            Variable::Argument(digit - 1)
        }
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

pub struct ExprBreadth {
    breadth_limit: usize,
    skeleton_idx: usize,
    expr_enum: ExprIdx,
    skeletons: Vec<ExprSkeleton>,
}

impl ExprBreadth {
    pub fn new(arg_count: usize, breadth_limit: usize) -> Self {
        let mut res = Self {
            breadth_limit,
            skeleton_idx: 0,
            expr_enum: ExprIdx::new(arg_count),
            skeletons: vec![Expr::Variable(())],
        };

        res.expr_enum.reset(&res.skeletons[0]);

        res
    }

    // NOTE: if we find a prettier way to do it -- do it asap, because currently
    // resetting stuff is very ugly looking
    pub fn next(&mut self) -> Option<Expr> {
        if self.skeletons.len() >= self.breadth_limit {
            return None;
        }

        // This loop looks a bit ass-backwards, but it is the best thing
        // to do, when generators aren't available yet.
        loop {
            let curr_skele = &self.skeletons[self.skeleton_idx];
            let attempt = self.expr_enum.next_expr(curr_skele);

            // Perhaps we haven't run out of expression for a single skelly?
            if attempt.is_some() {
                return attempt;
            }

            // Maybe we have more skellies to explore!
            self.skeleton_idx += 1;
            if let Some(skele) = self.skeletons.get(self.skeleton_idx) {
                self.expr_enum.reset(skele);
                continue;
            }

            // Aw, dang it.
            self.expand_holes();
        }
    }

    fn expand_holes(&mut self) {
        self.skeleton_idx = 0;

        let new_skeletons = self.skeletons.iter()
            .flat_map(Self::grow_skeleton)
            .collect();

        self.skeletons = new_skeletons;
        self.expr_enum.reset(&self.skeletons[0]);
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
            .map(|x| ExprSkeleton::Unop(x, Box::new(ExprSkeleton::Variable(()))));
        let binop_substs = all_binops.into_iter()
            .map(|x| ExprSkeleton::Binop(x, Box::new((ExprSkeleton::Variable(()), ExprSkeleton::Variable(())))));

        unop_substs.chain(binop_substs)
    }
}

pub struct BruteEnum {
    breadth: ExprBreadth,
}

impl Synthesizer for BruteEnum {
    fn build(var_count: usize) -> Self {
        Self {
            breadth: ExprBreadth::new(var_count, DEFAULT_BREADTH_LIMIT),
        }
    }

    fn learn(&mut self, _example: super::Example) {
        // A brute doesn't learn
    }

    fn next_expr(&mut self) -> Option<Expr> {
        self.breadth.next()
    }
}