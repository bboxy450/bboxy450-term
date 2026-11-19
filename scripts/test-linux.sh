#!/bin/sh
# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md

set -eu
if [ "${1:-}" != --inside ]; then
    source=$(pwd -P)
    case "$source" in
        "$HOME"/Library/*)
            # Dory hides ~/Library from the guest. Give it a disposable copy
            # of the exact tracked worktree bytes, including submodule sources.
            stage=$(mktemp -d "$HOME/bboxy450-term-linux.XXXXXXXX")
            trap 'rm -rf -- "$stage"' EXIT
            mkdir "$stage/source"
            git ls-files --cached --recurse-submodules -z > "$stage/paths"
            tar --null -T "$stage/paths" -cf "$stage/source.tar"
            tar -xf "$stage/source.tar" -C "$stage/source"
            source="$stage/source"
            ;;
    esac
    # Dory reserves the fixed name atomically; a second run fails with a
    # name conflict before it can build or use the target volume.
    # One rustc at a time keeps mux lib and test codegen within 4 GiB.
    dory run --rm --cpus 2 --memory 4g --name bboxy450-term-test-linux \
        --mount "type=bind,source=$source,target=/src,readonly" \
        --mount type=volume,source=bboxy450-term-linux-cargo-registry,target=/usr/local/cargo/registry \
        --mount type=volume,source=bboxy450-term-linux-target,target=/build \
        --workdir /src --env CARGO_TARGET_DIR=/build --env CARGO_BUILD_BUILD_DIR=/build \
        --env CARGO_BUILD_JOBS=1 --env RUST_TEST_THREADS=2 \
        --env CARGO_PROFILE_DEV_DEBUG=0 --env CARGO_PROFILE_TEST_DEBUG=0 \
        --env CARGO_PROFILE_DEV_OPT_LEVEL=1 --env CARGO_PROFILE_TEST_OPT_LEVEL=1 \
        --env RUSTUP_TOOLCHAIN=1.100.0 \
        rust:1.100.0-bookworm@sha256:ade39a710ef85f3506cffcfec3e8a24ba090badef1dff2fb798f71d878833fea \
        sh scripts/test-linux.sh --inside
    exit
fi

host=$(rustc -vV | sed -n 's/^host: //p')
test "$(rustc --version | cut -d ' ' -f 2)" = 1.100.0
# Run subprocess tests on the guest's native architecture. On ARM, build the
# required x86-64 Linux executable with a cross linker instead of emulating rustc.
set -- python3 zsh
case "$host" in
    x86_64-unknown-linux-gnu)
        checksum=b0959bc64232974224cf9fb22c915845398c0e8668fa7831eb5b43fb1eef781f ;;
    aarch64-unknown-linux-gnu)
        checksum=9164568e9840c6509177086828ac7d3178c2d8feb0658b4d2bff62fa620e98c8
        set -- "$@" gcc-x86-64-linux-gnu libc6-dev-amd64-cross ;;
    *)
        echo "unsupported Linux guest host: $host" >&2
        exit 1 ;;
esac
apt-get update -qq
apt-get install -y --no-install-recommends "$@"
nextest=0.9.150
archive="/tmp/cargo-nextest.tar.gz"
curl -fsSL --retry 3 -o "$archive" "https://github.com/nextest-rs/nextest/releases/download/cargo-nextest-$nextest/cargo-nextest-$nextest-$host.tar.gz"
printf '%s  %s\n' "$checksum" "$archive" | sha256sum -c -
tar -xzf "$archive" -C /usr/local/cargo/bin cargo-nextest
packages=$(cargo tree --locked --no-default-features -p bboxy450-term -e normal,build --prefix none)
if printf '%s\n' "$packages" | grep -Eq '^(bboxy450-render|wgpu|winit) v'; then
    echo "node dependency tree contains a renderer or windowing crate" >&2
    exit 1
fi
cargo build --locked --no-default-features -p bboxy450-term --bin bboxy450-term
cargo build --locked -p bboxy450-harness --bin bboxy450-harness
python3 -I scripts/test-node.py "$CARGO_TARGET_DIR/debug/bboxy450-term"
if [ "$host" != x86_64-unknown-linux-gnu ]; then
    rustup target add x86_64-unknown-linux-gnu
    export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER=x86_64-linux-gnu-gcc
    export CC_x86_64_unknown_linux_gnu=x86_64-linux-gnu-gcc
    cargo build --locked --no-default-features -p bboxy450-term --bin bboxy450-term \
        --target x86_64-unknown-linux-gnu
    readelf -h "$CARGO_TARGET_DIR/x86_64-unknown-linux-gnu/debug/bboxy450-term" | grep "X86-64"
fi

# Check the phase-one node boundaries. Plugin conformance, Dream and
# signed-source fixtures are exercised by their own host gates.
printf '[store]\ndir = "%s/nextest"\n' "$CARGO_TARGET_DIR" > /tmp/bboxy-nextest.toml
filter='package(=bboxy450-harness-host) |
        package(=bboxy450-keychain-file) |
        package(=bboxy450-desktop-notifications-linux) |
        (package(=bboxy450-term) & test(/^(cockpit::(owners|shell_path)|ctl)::/)) |
        (package(=bboxy450-term-mux) & test(/^(ctl_door|gui_capture|identity::(keychain|test_account)|mcp::bridge|seat_socket)::/))'
cargo nextest run --tool-config-file bboxy-linux:/tmp/bboxy-nextest.toml --locked --no-default-features --lib \
    -p bboxy450-term -p bboxy450-term-mux -p bboxy450-harness-host \
    -p bboxy450-keychain-file -p bboxy450-desktop-notifications-linux -E "$filter"
cargo nextest run --tool-config-file bboxy-linux:/tmp/bboxy-nextest.toml --locked -p bboxy450-harness --lib \
    -E "test(/^(backend|capability|gateway|mcp|protocol|runtime|store)::/)"
