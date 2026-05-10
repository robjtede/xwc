# xwc

`xwc` is a small `wc`-style command line tool for counting lines, bytes, and optionally words, characters, or maximum line length.

By default it prints line and byte counts with headings:

```console
$ printf 'one two\nthree\n' | xwc
lines  bytes
2      14
```

## Install

Recommended, using prebuilt release binaries and [cargo-binstall](https://github.com/cargo-bins/cargo-binstall):

```console
$ cargo binstall xwc
```

Fallback, building from source with Cargo:

```console
$ cargo --locked install xwc
```

## Usage

```console
$ xwc [OPTIONS] [FILE]...
```

When no files are provided, `xwc` reads from standard input. Use `-` as a file argument to read from standard input alongside file paths.

Options:

- `-l`, `--lines`: print only the newline count.
- `-c`, `--bytes`: print only the byte count.
- `-w`, `--words`: print only the word count.
- `-m`, `--chars`: print only the UTF-8 character count.
- `--longest-line`: print only the longest line length.
- `-A`, `--all`: print all counts.
- `-W`, `--include-words`: include the word count in the default output columns (slower).
- `-M`, `--include-chars`: include the UTF-8 character count in the default output columns.
- `--include-longest-line`: include the longest line length in the default output columns.
- `-h`, `--human-readable`: print byte counts with human-readable units and use the `size` heading.
- `--self-profile`: include per-input counting duration in the output.
- `-j`, `--jobs <N>`: set the worker count for multiple input files. By default, `xwc` starts parallel counting after 3 input files.
- `--glob <PATTERN>`: add files matching a glob pattern. Can be used more than once.
- `--sort-by <COLUMN>`: sort file rows by `lines`, `words`, `chars`, `max-line`, `bytes`, `file`, or `duration`.
- `--sort-order <ORDER>`: set output sort order to `asc` or `desc`.
- `--help`: print help.

Examples:

```console
$ xwc Cargo.toml src/main.rs
lines  bytes  file
8      174    Cargo.toml
420    10324  src/main.rs
428    10498  total

$ xwc --include-words Cargo.toml
lines  words  bytes  file
8      24     174    Cargo.toml

$ printf 'café\n' | xwc -m
5

$ printf 'one\nthree\ncafé\n' | xwc --longest-line
5

$ printf 'one\nthree\ncafé\n' | xwc --include-longest-line
lines  max-line  bytes
3      5         16

$ printf 'café\n' | xwc -M
lines  chars  bytes
1      5      6

$ printf 'café\n' | xwc --all
lines  words  chars  max-line  bytes
1      1      5      4         6

$ xwc -hc Cargo.toml
174B  Cargo.toml

$ xwc --glob 'src/*.rs'
lines  bytes  file
216    5266   src/lib.rs
613    15509  src/main.rs
829    20775  total
```

## Performance

Some basic `hyperfine` results `xwc` against `wc`:

| Case                             | `xwc` is ...    |
| -------------------------------- | --------------- |
| 4 KiB single file, lines + bytes | slightly slower |
| 1 MiB single file, lines only    | faster          |
| 64 MiB single file, bytes only   | slightly slower |
| 64 MiB single file, words        | faster          |
| 8 MiB UTF-8 file, chars          | much faster     |
| 16 x 64 KiB files, lines + bytes | tied            |
| 8 x 8 MiB files, lines + bytes   | much faster     |
| 8 x 8 MiB files, `xwc -j1`       | much faster     |

## Development

Just:

```console
$ just xwc --include-words Cargo.toml
```

Cargo:

```console
$ cargo run -- [OPTIONS] [FILE]...
```

Common development recipes:

```console
$ just fmt check test-no-coverage
```
