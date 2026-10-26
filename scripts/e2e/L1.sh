# coverage: full
# L1: real app input/text/frames and independent approved blur with negative controls.
. "$(dirname "$0")/lib.sh"
exec python3 -I "$(dirname "$0")/launcher.py"
