# Native notification evidence

Use the repository Makefile (`make help`) for all build/install/test/release work.
`make install` builds the release app, stamps its Cargo version/source revision,
signs with the approved Apple Development identity and installs in `~/Applications`.
`make install-unsigned` installs an ad-hoc-only debug bundle in `target/install-unsigned`;
it is for isolated tests, and `make smoke` runs it headless with its own file-custody
account. `UNSIGNED_INSTALL_DIR` and `INSTALL_DIR` override the destinations.
`make release` makes the signed app archive, checksum and detached GPG signature.
The old bundle script forwards to `make bundle`; it contains no signing recipe.

The signed bundle and installation targets require the approved Apple Development identity
`3943AF130A015B36824B7FBC86AA672E7B25F3C4` (team `UQB3368A84`). They fail if it
is unavailable. An ad-hoc signature fails notification authorization on this
machine; the identity-signed bundle works. Developer ID distribution signing
remains deferred.

Both bundles include `bboxy.icns`, generated from `.bboxy/images/bboxy.png`,
which is byte-identical to `~/.agents/identity/bboxy.png` (SHA256
`bc5cc774612f29a452967a5a4fa993292092a59b58dfd03dfb7ddb6b3286c370`).
The adjacent `.source-sha256` records that source digest. To regenerate on macOS
from the repository root:

```sh
icon_work=$(mktemp -d)
mkdir "$icon_work/bboxy.iconset"
for size in 16 32 128 256 512; do
    sips -z "$size" "$size" .bboxy/images/bboxy.png --out "$icon_work/bboxy.iconset/icon_${size}x${size}.png"
    double=$((size * 2))
    sips -z "$double" "$double" .bboxy/images/bboxy.png --out "$icon_work/bboxy.iconset/icon_${size}x${size}@2x.png"
done
iconutil -c icns -o packaging/macos/bboxy.icns "$icon_work/bboxy.iconset"
shasum -a 256 .bboxy/images/bboxy.png | cut -c1-64 > packaging/macos/bboxy.icns.source-sha256
rm -r "$icon_work"
```

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
The probe has a distinct stable bundle identifier. Record the bundle/binary
digest with the results. Cold-launch proof requires a post and exit in one PID,
then the click in a different PID; `deferred_landing` only records a tab switch.

The operator-present session on 2026-10-24 proved persisted permission,
replacement without duplicate delivered identifiers, and click while closed.
The first authorization timed out and returned Denied before later Granted;
it was not a clean single-Allow flow. The full committed raw proof is
[attention-physical-probe-2026-10-24.jsonl](../../.bboxy/runs/attention-physical-probe-2026-10-24.jsonl)
at `14bc2d49`, SHA256
`8b6d3935481ec31e5b3582dd5d7c4dfc555e065f6a254248085e637e4bdedad2`.
Lines 216/218 and 222 show PID 73566 posting and exiting; 224, 227 and 228 show
PID 74411 launching, receiving the click, and focusing seat 3 in tab 2.
