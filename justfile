_list:
    @just --list

# Run xwc with args
xwc *args:
    @cargo run --quiet -- {{ args }}

# Lint workspace with Clippy
clippy:
    cargo clippy --workspace --all-targets -- -D warnings

# Check Rust formatting
fmt-rust-check:
    cargo +nightly fmt -- --check

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
    just fmt-rust-check
    just clippy
    cargo machete --with-metadata

# Format project
fmt:
    just --unstable --fmt
    fd --type=file --hidden --extension=nix --exec-batch nixfmt
    fd --type=file --hidden --extension=md --extension=yml --exec-batch prettier --write
    fd --hidden --extension=toml --exec-batch taplo format
    cargo +nightly fmt

# Generate benchmark fixture files
[arg("size", long="size", help="fixture sizes, e.g. '1K 10M 500M'")]
[arg("block_size", long="block-size", help="reusable generation block size")]
[arg("chunk_size", long="chunk-size", help="simulated read chunk size for split fixtures")]
fixtures size="1K 10K 100K 1M 10M 100M 500M" chunk_size="65536" block_size="1048576":
    XWC_FIXTURE_SIZES='{{ size }}' XWC_CHUNK_SIZE='{{ chunk_size }}' XWC_BLOCK_SIZE='{{ block_size }}' benches/fixtures/generate.sh
