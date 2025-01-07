## Building

1. Make sure you have rust 1.81.0 installed with cargo
2. Make sure you have Z3 installed
    - On Linux it can be done this way
```sh
sudo apt-get install libz3-4
```
2. Cd into the project directory
3. Run `cargo build`

## Running

```
Usage: bitsynth.exe [OPTIONS]

Options:
  -t, --trace
  -v, --verbose
      --timeout <TIMEOUT>
  -c, --constraint <CONSTRAINT>
  -a, --arg <ARG>
      --solver <SOLVER>          [default: circuit] [possible values: brute, simple, circuit]
  -h, --help                     Print help
```