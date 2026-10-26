# coverage: full
# T1: the real launcher's keyboard path, PTYs, names and captured placements.
# Deterministic harness executables are isolated to this promise's private app.
. "$(dirname "$0")/lib.sh"
exec python3 -I -B "$(dirname "$0")/t1_launch.py"
