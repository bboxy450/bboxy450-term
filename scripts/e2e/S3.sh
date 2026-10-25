# coverage: partial; missing: 50 watched GUI panes with history; per-Playback and total retained heap bytes; paging hitch meter
# RSS or fifty separate tabs cannot stand in for fifty watched historical viewports.
. "$(dirname "$0")/lib.sh"
fail 'S3 metrics_unavailable: ctl exports no GUI history retained-heap or paging meter; no scale pass claimed'
