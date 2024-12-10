open Z3.BitVector

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

(* TODO: add > and < *)
type 'a bit_term =
  | Const of int32
  | Var of 'a
  | BinOp of { op:bit_binop; l: 'a bit_term; r: 'a bit_term }
  | UnOp of { op:bit_unop; x: 'a bit_term }

let bits_size = 32

let const_to_z3 ctx c = mk_numeral ctx c bits_size

let bit_unop_z3 ctx b =
  match b with
  | Neg -> mk_neg ctx
  | Not -> mk_not ctx
  | LShift n -> fun x -> mk_shl  ctx (const_to_z3 ctx (Int.to_string n)) x
  | RShift n -> fun x -> mk_lshr ctx (const_to_z3 ctx (Int.to_string n)) x

let bit_binop_z3 ctx b =
  match b with
  | Plus -> mk_add ctx
  | Minus -> mk_sub ctx
  | Xor -> mk_xor ctx
  | And -> mk_and ctx
  | Or -> mk_or ctx

let to_z3 ctx t =
  let rec go t =
    match t with
    | Const x -> const_to_z3 ctx (Int32.to_string x)
    | Var v -> v
    | BinOp { op; l; r } -> bit_binop_z3 ctx op (go l) (go r)
    | UnOp { op; x } -> bit_unop_z3 ctx op (go x)
  in
  go t