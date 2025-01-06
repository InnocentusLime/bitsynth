use std::{collections::HashMap, f32::consts::E, rc::Rc};

use z3::{ast::Ast, Solver};

use crate::expr::{BinopKind, Expr, ExprSkeleton, ExprVal, Variable, BITS_PER_VAL};

use super::Synthesizer;

#[derive(Clone, Debug)]
struct Connection<'ctx> {
    val: z3::ast::BV<'ctx>,
    loc: z3::ast::Int<'ctx>,
}

fn new_arg<'ctx>(z3: &'ctx z3::Context) -> Connection<'ctx> {
    Connection {
        val: z3::ast::BV::fresh_const(z3, "ca", BITS_PER_VAL),
        loc: z3::ast::Int::fresh_const(z3, "cal"),
    }
}

fn new_const<'ctx>(z3: &'ctx z3::Context) -> z3::ast::BV<'ctx> {
    z3::ast::BV::fresh_const(z3, "cc", BITS_PER_VAL)
}

fn new_input<'ctx>(z3: &'ctx z3::Context) -> Connection<'ctx> {
    Connection {
        val: z3::ast::BV::fresh_const(z3, "ci", BITS_PER_VAL),
        loc: z3::ast::Int::fresh_const(z3, "cil"),
    }
}

fn new_output<'ctx>(z3: &'ctx z3::Context) -> Connection<'ctx> {
    Connection {
        val: z3::ast::BV::fresh_const(z3, "co", BITS_PER_VAL),
        loc: z3::ast::Int::fresh_const(z3, "col"),
    }
}

fn new_result<'ctx>(z3: &'ctx z3::Context) -> Connection<'ctx> {
    Connection {
        val: z3::ast::BV::fresh_const(z3, "cr", BITS_PER_VAL),
        loc: z3::ast::Int::fresh_const(z3, "crl"),
    }
}

struct ComponentTemplate(Expr);

impl ComponentTemplate {
    fn spec<'ctx>(
        &self,
        z3: &'ctx z3::Context,
        output: &Connection<'ctx>,
        inputs: &[Connection<'ctx>],
        constants: &[z3::ast::BV<'ctx>],
    ) -> z3::ast::Bool<'ctx> {
        let expr = self.0.to_z3(
            z3,
            |_, idx| constants[idx].clone(),
            |_, idx| inputs[idx].val.clone(),
        );

        output.val._eq(&expr)
    }

    fn input_count(&self) -> usize {
        self.0.walk_expr(
            &mut |v| match v {
                Variable::Argument(_) => 1,
                _ => 0,
            },
            &mut |_, x| x,
            &mut |_, l, r| l + r,
            &mut |x| x,
        )
    }

    fn const_count(&self) -> usize {
        self.0.walk_expr(
            &mut |v| match v {
                Variable::UnknownConst => 1,
                _ => 0,
            },
            &mut |_, x| x,
            &mut |_, l, r| l + r,
            &mut |x| x,
        )
    }
}

struct Component<'ctx> {
    constants: Vec<z3::ast::BV<'ctx>>,
    inputs: Vec<Connection<'ctx>>,
    output: Connection<'ctx>,
}

impl<'ctx> Component<'ctx> {
    fn all_connections(&self) -> impl Iterator<Item = &Connection<'ctx>> {
        self.inputs.iter()
            .chain(std::iter::once(&self.output))
    }
}

struct LibrarySpec<'ctx> {
    components: Vec<Component<'ctx>>,
    args: Vec<Connection<'ctx>>,
    result: Connection<'ctx>,
}

struct Library {
    template: Vec<ComponentTemplate>,
    components: Vec<usize>,
}

impl Library {
    fn template_for(&self, comp_idx: usize) -> &ComponentTemplate {
        &self.template[self.components[comp_idx]]
    }

    fn spec<'ctx>(
        &self,
        arg_count: usize,
        z3: &'ctx z3::Context,
        solver: &z3::Solver<'ctx>,
    ) -> LibrarySpec<'ctx> {
        let loc_count = arg_count + self.components.len();
        let result = new_result(z3);
        let args = std::iter::from_fn(|| Some(new_arg(z3)))
            .take(arg_count)
            .collect::<Vec<_>>();
        let zero = z3::ast::Int::from_u64(&z3, 0);
        let loc_count = z3::ast::Int::from_u64(&z3, loc_count as u64);
        let arg_count = z3::ast::Int::from_u64(&z3, arg_count as u64);

        let components = Vec::<Component<'ctx>>::new();

        /* Lib and acyc constraint */
        for component in &self.components {
            let template = &self.template[*component];
            let component = Component {
                output: new_output(z3),
                constants: std::iter::from_fn(|| Some(new_const(z3)))
                    .take(template.const_count())
                    .collect(),
                inputs: std::iter::from_fn(|| Some(new_input(z3)))
                    .take(template.input_count())
                    .collect(),
            };

            solver.assert(&template.spec(
                z3,
                &component.output,
                &component.inputs,
                &component.constants,
            ));

            for inp in &component.inputs {
                solver.assert(&inp.loc.lt(&component.output.loc));
            }
        }

        /* Consistency constraint */
        for (i_x, x) in components.iter().enumerate() {
            for y in components.iter().skip(i_x + 1) {
                let x = &x.output;
                let y = &y.output;
                solver.assert(&!x.loc._eq(&y.loc));
            }
        }

        /* Domain constraints */
        for x in components.iter().map(|x| &x.output) {
            solver.assert(&arg_count.le(&x.loc));
            solver.assert(&x.loc.lt(&loc_count));
        }
        for x in components.iter().flat_map(|x| &x.inputs) {
            solver.assert(&zero.le(&x.loc));
            solver.assert(&x.loc.lt(&loc_count));
        }

        /* Equality constraint */
        let all_connections =
            components.iter()
                .flat_map(|x| x.all_connections())
                .chain(&args);
        for (i_x, x) in all_connections.into_iter().enumerate() {
            let all_connections =
                components.iter()
                    .flat_map(|x| x.all_connections())
                    .chain(&args);
            for y in all_connections.skip(i_x + 1) {
                solver.assert(
                    &(x.loc._eq(&y.loc))
                        .implies(&x.val._eq(&y.val))
                );
            }
        }

        LibrarySpec {
            args,
            components,
            result,
        }
    }
}

struct TestStorage {
    tests: Vec<(Vec<ExprVal>, ExprVal)>,
}

impl TestStorage {
    fn new() -> Self {
        Self {
            tests: Vec::new(),
        }
    }

    fn add_test(&mut self, args: Vec<ExprVal>, expected: ExprVal) {
        self.tests.push((args, expected))
    }

    fn spec<'ctx>(
        &self,
        z3: &'ctx z3::Context,
        args: &[Connection<'ctx>],
        result: &Connection<'ctx>,
        solver: &z3::Solver,
    ) {
        let mut ast_buff = Vec::with_capacity(args.len());

        for (inp, res) in &self.tests {
            debug_assert_eq!(inp.len(), args.len());

            ast_buff.clear();
            ast_buff.extend(
                inp.iter().zip(args)
                    .map(|(arg, conn)| conn.val._eq(&z3::ast::BV::from_i64(
                        z3,
                        *arg as i64,
                        BITS_PER_VAL
                    )))
            );

            let ref_buff = ast_buff.iter().collect::<Vec<_>>();
            let expected = z3::ast::BV::from_i64(
                z3,
                *res as i64,
                BITS_PER_VAL
            );
            let cond = z3::ast::Bool::and(z3, &ref_buff)
                .implies(&result.val._eq(&expected));

            solver.assert(&cond);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum ComponentIdx {
    Common(usize),
    Argument(usize),
}

pub struct CircuitEnum<'ctx> {
    arg_count: usize,
    solver: z3::Solver<'ctx>,
    z3: &'ctx z3::Context,
    lib: Library,
    tests: TestStorage,
}

impl<'ctx> CircuitEnum<'ctx> {
    fn synth_expr(&self) -> Expr {
        let (model, lib_spec) = self.synth_circuit()
            .expect("Can't handle lack of model");

        self.circuit_model_to_expr(&lib_spec, &model)
    }

    fn synth_circuit(&self) -> Option<(z3::Model<'ctx>, LibrarySpec<'ctx>)> {
        let mut model = None;

        self.solver.push();
        let lib_spec = self.prepare_spec();
        if self.solver.check() == z3::SatResult::Sat {
            model = Some((
                self.solver.get_model().unwrap(),
                lib_spec
            ));
        }
        self.solver.pop(1);

        model
    }

    fn prepare_spec(&self) -> LibrarySpec<'ctx> {
        let lib_spec = self.lib.spec(
            self.arg_count,
            self.z3,
            &self.solver,
        );

        self.tests.spec(
            self.z3,
            &lib_spec.args,
            &lib_spec.result,
            &self.solver,
        );

        lib_spec
    }

    fn learn(&mut self, args: Vec<ExprVal>, res: ExprVal) {
        self.tests.add_test(args, res);
    }

    fn circuit_model_to_expr(
        &self,
        lib_spec: &LibrarySpec<'ctx>,
        model: &z3::Model<'ctx>,
    ) -> Expr {
        let start_loc_idx = model.get_const_interp(
            &lib_spec.result.loc,
            )
            .unwrap()
            .as_u64()
            .unwrap() as usize;
        let mut component_idx = vec![
            ComponentIdx::Argument(0); lib_spec.components.len() + lib_spec.args.len()
        ];

        for (idx, component) in lib_spec.components.iter().enumerate() {
            let loc = model.get_const_interp(&component.output.loc)
                .unwrap()
                .as_u64()
                .unwrap() as usize;
            component_idx[loc] = ComponentIdx::Common(idx);
        }

        for (idx, arg) in lib_spec.args.iter().enumerate() {
            let loc = model.get_const_interp(&arg.loc)
                .unwrap()
                .as_u64()
                .unwrap() as usize;
            component_idx[loc] = ComponentIdx::Argument(idx);
        }

        self.build_expr_from_model_rec(
            start_loc_idx,
            &component_idx,
            lib_spec,
            model,
        )
    }

    fn build_expr_from_model_rec(
        &self,
        loc: usize,
        component_idx: &[ComponentIdx],
        lib_spec: &LibrarySpec<'ctx>,
        model: &z3::Model<'ctx>,
    ) -> Expr {
        let idx = component_idx[loc];
        let comp_idx = match idx {
            ComponentIdx::Common(x) => x,
            ComponentIdx::Argument(arg) => {
                return Expr::Variable(
                    Variable::Argument(arg)
                )
            },
        };

        let mut const_idx = 0;
        let template_idx = self.lib.template_for(comp_idx);
        let component = &lib_spec.components[comp_idx];

        template_idx.0.walk_expr(
            &mut |v| match v {
                Variable::UnknownConst => {
                    let c = &component.constants[const_idx];
                    let val = model.get_const_interp(c)
                        .unwrap()
                        .as_i64()
                        .unwrap() as ExprVal;
                    const_idx += 1;

                    Expr::Variable(Variable::Const(val))
                },
                Variable::Const(x) => Expr::Variable(Variable::Const(*x)),
                Variable::Argument(inp) => {
                    let inp = &component.inputs[*inp];
                    let loc = model.get_const_interp(
                            &inp.loc
                        )
                        .unwrap()
                        .as_u64()
                        .unwrap() as usize;

                    self.build_expr_from_model_rec(
                        loc,
                        component_idx,
                        lib_spec,
                        model,
                    )
                },
            },
            &mut |kind, e| {
                Expr::Unop(kind, Rc::new(e))
            },
            &mut |kind, l, r| {
                Expr::Binop(kind, Rc::new(l), Rc::new(r))
            },
            &mut |x| x,
        )
    }
}

fn default_lib() -> Library {
    let template = vec![
        ComponentTemplate(Expr::Binop(
            BinopKind::And,
            Rc::new(Expr::Variable(Variable::Argument(0))),
            Rc::new(Expr::Variable(Variable::Argument(1))),
        )),
        ComponentTemplate(Expr::Binop(
            BinopKind::Or,
            Rc::new(Expr::Variable(Variable::Argument(0))),
            Rc::new(Expr::Variable(Variable::Argument(1))),
        )),
        ComponentTemplate(Expr::Binop(
            BinopKind::Xor,
            Rc::new(Expr::Variable(Variable::Argument(0))),
            Rc::new(Expr::Variable(Variable::Argument(1))),
        )),
        ComponentTemplate(Expr::Binop(
            BinopKind::Minus,
            Rc::new(Expr::Variable(Variable::Argument(0))),
            Rc::new(Expr::Variable(Variable::Argument(1))),
        )),
        ComponentTemplate(Expr::Binop(
            BinopKind::Minus,
            Rc::new(Expr::Variable(Variable::Argument(0))),
            Rc::new(Expr::Variable(Variable::Const(1))),
        )),
        ComponentTemplate(Expr::Binop(
            BinopKind::Plus,
            Rc::new(Expr::Variable(Variable::Argument(0))),
            Rc::new(Expr::Variable(Variable::Const(1))),
        )),
        ComponentTemplate(Expr::Binop(
            BinopKind::ShrA,
            Rc::new(Expr::Variable(Variable::Argument(0))),
            Rc::new(Expr::Variable(Variable::UnknownConst)),
        )),
    ];

    Library {
        template,
        components: vec![
            0, 0, 0,
            1, 1, 1,
            2, 2, 2,
            3, 3, 3,
            4, 5, 6,
        ],
    }
}

impl<'ctx> Synthesizer<'ctx> for CircuitEnum<'ctx> {
    fn build(z3: &'ctx z3::Context, var_count: usize, _depth_limit: usize) -> Self {
        Self {
            arg_count: var_count,
            solver: z3::Solver::new(z3),
            z3,
            lib: default_lib(),
            tests: TestStorage::new(),
        }
    }

    fn bad_cand(&mut self, _cand: &Expr, args: Vec<ExprVal>, expected: ExprVal) {
        self.tests.add_test(args, expected);
    }

    fn next_expr(&mut self) -> Option<Expr> {
        Some(self.synth_expr())
    }
}