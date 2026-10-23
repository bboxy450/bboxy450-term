# Identity custody on startup

Production callers use `Logic::start_custodied`. Keychain operations run on an
identity IO worker with native user interaction disabled. An access prompt,
denied access or a locked item yields `Pending(KeychainInteractionRequired)`;
an operation that has not returned within two seconds yields
`Pending(KeychainTimedOut)`. Terminals and the GUI still start, messaging is
held, and the existing 30-second custody retry remains in effect.

All native Keychain instances share one in-flight guard. A stuck securityd call
can leave one worker blocked until it returns or the process exits; subsequent
requests time out without adding workers. A timed-out create may still finish
its atomic Keychain add. Recovery reads the item and checks the pinned public
key, so a timeout never replaces an issuer or creates a secret file.

Spawned process tests can set `BBOXY_TEST_CUSTODY=memory` on the child command.
The selector and memory-key implementation are compiled only with
`cfg(debug_assertions)`. Release code does not read this variable. It selects a
single process-owned in-memory `DeviceKeyStore` shared by identity startup,
retry and `local_dev_issuer`; it never opens the real Keychain. Keys disappear
when the process exits and its card/status label is `memory custody (debug)`.
Without the explicit switch, debug builds still select Keychain custody.
Use fresh run state for signing tests; reopening a pin
from a different process holds with the existing missing/mismatched-key reason.
The switch does not override an explicitly injected test store or rewrite
existing issuer history.
