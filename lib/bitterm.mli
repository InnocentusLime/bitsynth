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

val t_neg : 'a bit_term -> 'a bit_term

val t_not : 'a bit_term -> 'a bit_term

val t_shl : int -> 'a bit_term -> 'a bit_term

val t_shr : int -> 'a bit_term -> 'a bit_term

val t_plus : 'a bit_term -> 'a bit_term -> 'a bit_term

val t_minus : 'a bit_term -> 'a bit_term -> 'a bit_term

val t_xor : 'a bit_term -> 'a bit_term -> 'a bit_term

val t_and : 'a bit_term -> 'a bit_term -> 'a bit_term

val t_or : 'a bit_term -> 'a bit_term -> 'a bit_term