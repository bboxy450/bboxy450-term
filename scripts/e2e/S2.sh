# coverage: partial; missing: ring-level gap production not exercised; GUI historical cell bytes; headless gap-marker pixels
. "$(dirname "$0")/lib.sh"
python3 -I "$(dirname "$0")/ui.py" S2
