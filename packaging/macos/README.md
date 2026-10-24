# Native notification evidence

`sh scripts/bundle-macos.sh debug` builds the ordinary app bundle. `release`
packages release code, with production identity custody; never use that for
fixtures. Bare `cargo run` remains bare and logs the missing bundle once.

`sh scripts/notification-probe-macos.sh` builds a separate debug-only fixture
bundle. It starts no logic, PTY or Keychain access. The facade's opt-in `probe`
feature only reads native delivered identifiers and is absent in release.

The Oracle schedules the operator's session. The executor does not drive the
desktop or a permission dialog. Open the printed probe `.app` in that session:

1. Press **A**, allow notifications, then **A** again if the bounded call timed
   out while the dialog was open. The second result must be `Granted`. Quit
   with **Q**, relaunch the same bundle, and press **A**: no second prompt.
2. Press **P**. Wait until the first fixture is delivered, then **C**: the
   actual delivered list must contain `00000000000000000000000003` once.
   Press **R**, wait for replacement, then **C**: still one occurrence, with
   the replacement body visible. **D**, then **C**, must show its removal.
3. Press **P** again and wait for delivery, then **Q**. Click that delivered
   notification while the probe process is absent. A new process must log
   `clicked` and `focused`, select fixture seat 3 in tab 2, and raise its window.

Logs append to the current user's temporary directory at
`bboxy450-notification-probe/proof.jsonl`, including process ids across launch.
The bundle is ad-hoc signed with a distinct stable probe identifier. Record the
bundle/binary digest with the results. An unopened bundle or an `Ok` from
scheduling is not evidence of delivery or delivered-id replacement. Send the
three actual results to the Architect for the spec's acceptance record.
