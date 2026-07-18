# herdr-review dev tasks — run `just <task>` (https://github.com/casey/just)

# default: list tasks
default:
    @just --list

# format the code
fmt:
    cargo fmt --all

# check formatting (CI parity)
fmt-check:
    cargo fmt --all --check

# lint with clippy, warnings as errors (CI parity)
lint:
    cargo clippy --all-targets --all-features -- -D warnings

# run the test suite
test:
    cargo test --all-features

# build (debug)
build:
    cargo build

# run the sidebar in the current repo
run:
    cargo run

# build release and install the binary into bin/ for `herdr plugin link`
install:
    cargo build --release
    mkdir -p bin
    ./scripts/swap-binary.sh target/release/herdr-reviewr bin/herdr-reviewr

# build release and swap it into the GitHub-installed plugin for local QA (docs/qa-install.md)
qa-install:
    cargo build --release
    ./scripts/qa-install.sh

# everything CI runs, locally
ci: fmt-check lint test
    cargo build --release
