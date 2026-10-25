# coverage: partial; missing: amber mark pixel colour; operator DM native delivery; non-operator delayed delivery and exactly-once release; mute journal signer
# K2: Cmd-M changes only the idle focused seat through the native UI path.
. "$(dirname "$0")/lib.sh"
exec python3 -I "$(dirname "$0")/ui.py" K2
