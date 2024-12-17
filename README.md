## Getting Started With BitSynth

1. Install opam: [link](https://opam.ocaml.org/doc/Install.html)
2. With opam, install OCaml version 5.1.0
```
opam switch create 5.1.0
```
3. Install the following packages

```
opam install dune
opam install z3
```

## Bulding Bitsynth

To build all the facilities just run

```
dune build
```

## Running The Program

To run bitsynth, execute the following command

```
dune exec bitsynth
```