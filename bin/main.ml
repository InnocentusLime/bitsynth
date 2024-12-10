open Bitsynth

let () =
  let cfg = [("model", "true"); ("proof", "false")] in
  let ctx = Z3.mk_context cfg in
  let x = Z3.Expr.mk_const ctx (Z3.Symbol.mk_string ctx "x") (Z3.BitVector.mk_sort ctx 32) in
  let y = Z3.Expr.mk_const ctx (Z3.Symbol.mk_string ctx "y") (Z3.BitVector.mk_sort ctx 32) in
  let t1 = Bitterm.t_and (Bitterm.Var x) (Bitterm.Var y)
  and t2 =
    Bitterm.t_not (
      Bitterm.t_or
        (Bitterm.t_not (Bitterm.Var x))
        (Bitterm.t_not (Bitterm.Var y))
    )
  in
  let et1 = Bitterm.to_z3 ctx t1 in
  let et2 = Bitterm.to_z3 ctx t2 in
  let s = Z3.Solver.mk_solver ctx None in
  let st = Z3.Solver.check s [Z3.Boolean.mk_not ctx (Z3.Boolean.mk_eq ctx et1 et2)] in
    match st with
    | Z3.Solver.SATISFIABLE -> Printf.printf "sat (oops)\n"
    | Z3.Solver.UNSATISFIABLE -> Printf.printf "unsat (all good)\n"
    | Z3.Solver.UNKNOWN -> Printf.printf "I don't know (well, shit)\n"