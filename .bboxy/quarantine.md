# Flaky quarantine (#321 rule 4)

A test is listed here only with an owner, a task and a date. It still runs everywhere. When a run's
only failures are listed tests, each is rerun once alone by `scripts/quarantine-test.sh`; passing alone
does not fail the run, failing alone does. An entry leaves when fixed or after seven days, whichever is
first; more than five entries is a review, not a fact of life. The name column is matched exactly
against cargo's `test <name> ... FAILED` line.

| Test | Crate | Owner | Task | Since | Expires | Evidence |
|---|---|---|---|---|---|---|
| `fifty_seats_share_one_supervisor_poller` | bbt-supervisor | Sable | spawn ENOENT under load | 2026-10-25 | 2026-11-01 | gate 65927a6c red under three-slot load; 5/5 alone |
| `durable_gui_survives_real_logic_kill_keeps_unread_and_services_pty_controls` | bbt-mux | Sable | gui_events.rs:251 socket EOF after logic kill | 2026-10-25 | 2026-11-01 | Nyx and Aster prechecks red; 40/40 alone on tip and main |
