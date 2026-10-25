---
id: 01M5QNKAYJD3YCDAKVANXAMFGJ
type: decision
number: 282
title: "the whole app tests itself unattended: an automation identity in file custody, a headless app with a real GUI client and no window, one e2e runner command, and nothing of the night built or signed at night; the one operator act is one root signature, once"
status: decided
decided: "2026-10-24"
operator:
  - "well all the smoke tests and agent tests should not require me here, its already morning, you worked for 14hours, we need to adjust, ethire app should be automatically testable and smoket estable automatically, thats the core"
links:
  - { to: "[[e2e-acceptance]]", rel: decides }
  - { to: "[[ctl-cli]]", rel: decides }
  - { to: "[[id1-seat-chain]]", rel: builds-on }
  - { to: "[[d281-custody-facade-and-automation-identity]]", rel: builds-on }
  - { to: "[[d259-overnight-north-star]]", rel: amends }
  - { to: "[[d270-ctl-screen-input-wait]]", rel: builds-on }
  - { to: "[[e2e-runs-unattended]]", rel: constrains }
  - { to: "[[mantras]]", rel: mantra }
created: "2026-10-24T19:18:29Z"
updated: "2026-10-25T12:18:12Z"
---
# Decision 282: the app tests itself, unattended

Decided by the operator on the morning of 2026-10-24, verbatim: "well all the smoke tests and agent tests should not require me here, its already morning, you worked for 14hours, we need to adjust, ethire app should be automatically testable and smoket estable automatically, thats the core". Written by the Architect at the Oracle's ask; amends #259 (the north star) with the part it was missing: no operator in the loop.

**Alternatives.** (a) Keep the night as it ran: the operator present for the Keychain, the trust prompts and the display. (b) Four pieces and one remaining act. **An automation identity** (#281): a dedicated e2e device enrolled once under the operator's account, its key in the `-file` custody backend the harness creates and opens itself, never the login Keychain, selected by `--unattended` in release-equivalent builds; everything it signs says `issued_by: automation`. **A headless app**: `ctl app start --headless --unattended --ctl` runs supervisor, logic and a real GUI client with no window (the capture renderer drives the same model), so a locked display, no display or a night run all work and smoke has no `NSWindow` dependency. **One runner**: `bboxy450-term e2e night [--group]` starts the headless app, spawns worker testers through `ctl`, delegates every e2e record, answers known harness prompts through recipes, waits, collects `verify_result`s, updates the checklist, and exits `0` only when the checklist is green or every non-green row names its open task (the amber rule of [[e2e-acceptance]] §4), `1` otherwise with the red rows listed. **Nothing is built or signed at night**: the runner runs the identity-signed bundle the day produced, because the codesigning identity lives in the login Keychain and a locked Keychain would hold it. (c) Unlock the operator's Keychain from the harness.

**Choice.** (b).

**Reason.** The operator's words are the whole reason: a test that needs a human present is not a test, it is a meeting. Every prompt the night hit has an unattended answer that keeps its boundary: custody by a scoped automation key rather than the operator's; Codex's hook review by the recipe with the exact-count guard (#270); the notification authorization persisted once and proven (#45); the display by a headless client. (c) breaks custody, and (a) is the fourteen hours.

**The one operator act that remains.** Enrolling the automation device: one root signature on its `DeviceGrant`, once, with the scope written in the grant (e2e writes and `ctl` under the operator principal). After that, nothing: no Keychain dialog (file custody), no trust prompt (recipes), no notification prompt (persisted), no display (headless), no build or signature (pre-built). Revocation is one root signature too.

**Order.** U-0 the `-file` custody backend and the automation identity with `--unattended` (#281). U-1 `ctl app start --headless` with the real GUI client (C-1 of [[ctl-cli]]). U-2 the runner. U-3 the invariant [[e2e-runs-unattended]] in the slow tier of local CI. Executors start in parallel; the runner is the only piece that waits for the other three.

**Complete (2026-10-25).** U-0 file custody and per-run test accounts (Kestrel, 6ea6b23e), U-1 the headless app (Aster, afff4758), U-2 the night runner (Seraph, 7d4d599e), U-3 the invariant [[e2e-runs-unattended]] active on `scripts/e2e-unattended.sh` (Seraph, 6dfbf569). The first unattended night ran amber on 2026-10-24 with every partial tracked; the operator's words of that morning are a passing check.

Owner: Corvin Duskmantle <ARCHITECT>.

**Addendum (Oracle, same day): the lifecycle wire.** `app start` replies only after backend and offscreen render readiness, with run directory, mode, `unattended` and generation and never the capability; `app stop` is an authenticated relay to an orderly `Session` shutdown with no PID-kill fallback (`endpoint_unavailable`, exit 4, otherwise); `app status` reports liveness, mode, readiness and generation and never a secret ([[ctl-cli]] §1b). Aster's read-only plan maps to it.

**Amended the same morning (operator, verbatim: "no, e2e still requires my action").** The one remaining operator act was one too many. **e2e never touches the operator's account or identity.** The runner creates a **throwaway test account per run**: its own root key, generated by the harness in file custody under the run's private directory, and the e2e device under it, with its own run directory and data directory; everything the night signs is signed by that test account and labelled `issued_by: test-account:<run>`; the operator's real account is only for real work; **zero operator acts, ever, for testing.** The automation identity under the operator's account (#281) stays as an *optional* path for unattended real work, not a requirement for e2e. **Verdicts are deterministic:** a promise passes or fails by `ctl` scripts, exit codes and screen and record assertions, never by an agent reporting pass; real agents are tested only for agent behaviour (wake, report, reply, orient on the first turn), verified from the app's own records; their intuitiveness feedback is advisory, filed as comments, and is never a verdict. Rejected: any enrollment step for testing; an agent's self-report as a pass.
