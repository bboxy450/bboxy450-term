# coverage: partial; missing: 50 watched GUI panes with history; paging hitch meter
. "$(dirname "$0")/lib.sh"
python3 -I "$(dirname "$0")/ui.py" S3
