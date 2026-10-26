#!/bin/sh
# Exact real boundaries for bboxybook; append new selectors here.
set -eu
cargo test -p bboxybook --locked --test documents every_tracked_bboxy_markdown_lays_out_in_both_themes_without_unclipped_overflow -- --ignored --exact --nocapture
