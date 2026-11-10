---
type: decision
title: TermC1 owns automatic post-close work and suppresses legacy D0 and IR advice jobs
id: 01M6NRNZQSXEWF9M1BD7CRZB74
updated: 2026-11-05T11:49:35.940Z
created: 2026-11-05T11:49:35.940Z
number: 93377961364464
created_by: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
updated_by: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
authorship:
  signer: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
  kind: seat
  public_key: UgGFd6fs0Ln9gJXlN2fw8kUmonpBkr4IBn/U5N0uOrY=
  process_generation: '1'
provenance:
  digest: sha256:18f35fe2d2f8bc3487df724a1c652af3c02ebb9274d9004b5c495f39e003dd89
  frontmatter_digest: sha256:311c462f8cd08ed1b8cfbf16dcec1249ccfc98cc5635a2e9342597de468e412a
  signer: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
  sig: 6jVO7iJJ1TMndURf8PEZUKY0k0BDhgwjrOxqf9hGNszH18kY4F5AhOZxmcpIVic/oUdlbuk2Lld0076X5KJPCw==
  log: .bboxy/log/2026-11-05.jsonl#134686
---
# One bounded post-close owner for TermC1

Decision: in explicit DreamOwner::TermC1 mode, the durable SessionClosed notification is followed only by the managed C1 owner path. Suppress both automatic legacy D0 replay and automatic IR advice scanning through one typed gate in dream/mod.rs used by engine, ACP and import-watch hooks. Legacy standalone/manual behavior is preserved.

Evidence: engine/mod.rs:450-455 and corresponding ACP/watch hooks retain an unconditional ir_scan::session_closed call after the D0 gate. ir_scan.rs:455-463 uses shared blocking work, :503-512 expands a whole advertised blob, and :632-651 detaches the close job. This separate scanner could defeat the total managed post-close smoothness guarantee; it is not itself duplicate D0 work.

Reason: C1's worker deliberately performs selected-input event-only inspection without schema advice, Git/KB discovery or a broader scan. Leaving another unbudgeted scanner active would contradict the operator's explicit post-session background CPU/foreground requirements. Reengineering the IR scanner is outside C1; automatic IR advice is excluded in this opt-in managed mode until separately bounded/integrated. No scanner implementation edit is authorized by this choice.

Verification: use a real managed source close containing a large JSON blob while foreground traffic runs. Assert both automatic legacy D0 and IR advice hooks remain uninvoked in TermC1, one managed closed-source job is admitted, and control/input/message/frame bounds hold. Preserve explicit Legacy hook witnesses and durable SessionClosed behavior.

Governs [[specs/dream-term-c1]] and [[interfaces/dream-term-c1]]. Producer owns the shared typed close gate; consumer sets it only after verifying the matching worker capability. Existing source paths, wire format and Make gates are unchanged.