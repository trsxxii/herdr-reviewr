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
    # Replace with a fresh inode, then ad-hoc re-sign: macOS SIGKILLs a binary whose signature
    # an in-place overwrite invalidated, so a plain `cp` over the old binary makes the pane die
    # at launch on Apple Silicon. No-op on Linux.
    rm -f bin/herdr-reviewr
    cp target/release/herdr-reviewr bin/herdr-reviewr
    [ "$(uname)" = "Darwin" ] && codesign --force --sign - bin/herdr-reviewr || true

# The rm + cp + codesign order below is load-bearing: overwriting the installed binary in
# place invalidates its cached code signature and macOS SIGKILLs every launch, so the pane
# opens dead with no error. A fresh inode plus an ad-hoc re-sign avoids that. The previous
# binary is kept once as herdr-reviewr.release-backup (`just qa-restore` brings it back).

# build release and swap it into the installed GitHub plugin, for QA against the live pane
qa-install:
    #!/usr/bin/env sh
    set -eu
    cargo build --release
    bin="$(ls -d "$HOME"/.config/herdr/plugins/github/persiyanov.reviewr-*/bin/herdr-reviewr | head -1)"
    [ -f "$bin.release-backup" ] || cp "$bin" "$bin.release-backup"
    rm "$bin"
    cp target/release/herdr-reviewr "$bin"
    [ "$(uname)" = "Darwin" ] && codesign --force --sign - "$bin" || true
    echo "installed local build at $bin"

# restore the released binary the last `just qa-install` replaced
qa-restore:
    #!/usr/bin/env sh
    set -eu
    bin="$(ls -d "$HOME"/.config/herdr/plugins/github/persiyanov.reviewr-*/bin/herdr-reviewr | head -1)"
    cp "$bin.release-backup" "$bin"
    echo "restored release binary at $bin"

# everything CI runs, locally
ci: fmt-check lint test
    cargo build --release
