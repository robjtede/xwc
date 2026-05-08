# vwc

`vwc` is a small `wc`-style command line tool for counting lines, bytes, and optionally words.

By default it prints line and byte counts with headings:

```console
$ printf 'one two\nthree\n' | vwc
lines  bytes
2      14
```

## Usage

```console
$ vwc [OPTIONS] [FILE]...
```

When no files are provided, `vwc` reads from standard input. Use `-` as a file argument to read from standard input alongside file paths.

Options:

- `-l`, `--lines`: print the newline count.
- `-c`, `--bytes`: print the byte count.
- `-w`, `--words`: print only the word count.
- `-W`, `--include-words`: include the word count in the default output columns.
- `-h`, `--human-readable`: print byte counts with human-readable units and use the `size` heading.
- `--help`: print help.

Examples:

```console
$ vwc Cargo.toml src/main.rs
lines  bytes  file
8      174    Cargo.toml
420    10324  src/main.rs
428    10498  total

$ vwc --include-words Cargo.toml
lines  words  bytes  file
8      24     174    Cargo.toml

$ vwc -hc Cargo.toml
174B  Cargo.toml
```

## Development

Run the binary through Cargo:

```console
$ cargo run -- [OPTIONS] [FILE]...
```

Or use the Just proxy recipe:

```console
$ just vwc --include-words Cargo.toml
```

Common development recipes:

```console
$ just fmt
$ just check
$ just test-no-coverage
```
