# coverage: full
# harness: real PTY OSC fixture, no native model/client claim
# T8: silent inferred idle, valid reported state, rejected malformed reports and actual grade pixels.
. "$(dirname "$0")/lib.sh"
exec python3 -I -B "$(dirname "$0")/t8_reports.py"
