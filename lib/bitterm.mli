type bit_unop =
  | Neg
  | Not
  | LShift of int
  | RShift of int

type bit_binop =
  | Plus
  | Minus
  | Xor
  | And
  | Or

type 'a bit_term =
  | Const of int32
  | Var of 'a
  | BinOp of { op:bit_binop; l: 'a bit_term; r: 'a bit_term }
  | UnOp of { op:bit_unop; x: 'a bit_term }

val to_z3 : Z3.context -> Z3.Expr.expr bit_term -> Z3.Expr.expr