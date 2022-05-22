# gpn-mazing Bot

This is my [gpn-mazing](https://github.com/freehuntx/gpn-mazing) bot implementation from Gulasch-Programmier-Nacht 2022 (GPN 20), known from the *michael-1.0* bot.

It uses some kind of depth-first search in the maze, with heuristics for finding the best decision at each crossing.
In addition, it checks if the goal is even reachable via the unexplored-fields from the next field before going there, effectively skipping all areas which cannot lead to the goal anymore.

I also implemented cutting further unnecessary sub-trees from the depth-first search, based on missing improvement of the heuristic score, but it resulted in to many false-positives, i.e. skipping the correct path to the goal.

## Running

Compile and run with
```bash
cargo run
```

The program accepts one command line argument with the path of the config file.
If not given, a `config.toml` in the current working directory is expected.
An example config file is given in [config.example.toml](config.example.toml).

To enable logging, set the `RUST_LOG` environment variable to the desired log level (e.g. `RUST_LOG=info`).
