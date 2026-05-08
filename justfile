_list:
    @just --list --list-submodules

# Run vwc with args.
vwc *args:
    @cargo run --quiet -- {{ args }}
