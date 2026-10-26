#!/bin/sh
# Exact bbt-rt native boundaries and diagnostic measurements; no speed assertions.
set -eu
cargo test --target-dir "${BBOXY_CARGO_TARGET_DIR:-target}" -p bboxy450-rt --locked --test runtime million_messages_per_second_never_wait_for_a_consumer -- --ignored --exact --test-threads=1 --nocapture
cargo test --target-dir "${BBOXY_CARGO_TARGET_DIR:-target}" -p bboxy450-rt --locked --test runtime real_process_boundary -- --ignored --exact --test-threads=1 --nocapture
cargo test --target-dir "${BBOXY_CARGO_TARGET_DIR:-target}" -p bboxy450-rt --locked --test r5_instruments real_two_second_file_stall_reports_native_measurements -- --ignored --exact --test-threads=1 --nocapture
