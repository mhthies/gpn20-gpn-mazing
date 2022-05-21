# gpn-mazing Bot

This is my [gpn-mazing](https://github.com/freehuntx/gpn-mazing) bot implementation from Gulasch Programmier Nacht 20.

It uses some kind of depth-first search in the maze, with heuristics for finding the best decision at each crossing.


## Running

Compile and run with
```bash
cargo run
```

The program accepts one command line argument with the path of the config file.
If not given, a `config.toml` in the current working directory is expected.
An example config file is given in [config.example.toml](config.example.toml).

To enable logging, set the `RUST_LOG` environment variable to the desired log level (e.g. `RUST_LOG=info`).
