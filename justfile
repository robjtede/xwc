set positional-arguments := true

_list:
    @just --list

# Run xwc with args
xwc *args:
    @cargo run --quiet -- {{ args }}

# Lint workspace with Clippy
clippy:
    cargo clippy --workspace --all-targets -- -D warnings

# Test workspace
test:
    cargo nextest run --workspace --all-features

# Test workspace and generate Codecov coverage file
test-coverage-codecov toolchain="":
    cargo {{ toolchain }} llvm-cov --workspace --all-features --codecov --output-path codecov.json

# Test workspace and generate LCOV coverage file
test-coverage-lcov toolchain="":
    cargo {{ toolchain }} llvm-cov --workspace --all-features --lcov --output-path lcov.info

# Document workspace
doc *args:
    RUSTDOCFLAGS="--cfg=docsrs" cargo +nightly doc --no-deps --workspace --all-features {{ args }}

# Document workspace and watch for changes
doc-watch: (doc "--open")
    cargo watch -- RUSTDOCFLAGS="--cfg=docsrs" cargo +nightly doc --no-deps --workspace --all-features

# Check project
check:
    just --unstable --fmt --check
    fd --type=file --hidden --extension=nix --exec-batch nixfmt --check
    fd --type=file --hidden --extension=md --extension=yml --exec-batch prettier --check
    fd --hidden --extension=toml --exec-batch taplo format --check
    fd --hidden --extension=toml --exec-batch taplo lint
    cargo +nightly fmt -- --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo machete --with-metadata

# Format project
fmt:
    just --unstable --fmt
    fd --type=file --hidden --extension=nix --exec-batch nixfmt
    fd --type=file --hidden --extension=md --extension=yml --exec-batch prettier --write
    fd --hidden --extension=toml --exec-batch taplo format
    cargo +nightly fmt
