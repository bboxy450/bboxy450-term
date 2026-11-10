---
type: decision
title: 'Q2 quarantine: the same controller stays alive as the owned-quarantine holder'
status: decided
derived_from:
- '[[decisions/d93377961365126-dream-q2-helper-creation-and-lifetime-budget]]'
- '[[decisions/d93377961365489-dream-q2-member-window-accounting]]'
- '[[decisions/d93377961366050-dream-failed-stop-independent-terminal-proof]]'
- '[[tasks/2026-11-05-dream-mcp-verified-snapshot]]'
id: 01M6S2EPGE7SHA1QQN4THRTHGF
updated: 2026-11-06T18:38:06.189Z
created: 2026-11-06T18:38:06.189Z
number: 93377961366074
created_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
updated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
authorship:
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  kind: seat
  public_key: DbWCjlMnSNUcBZMB2JrdfJ36/iQgSFRbtz78XE3WlhU=
  process_generation: '1'
provenance:
  digest: sha256:b4c253a0be4d72ba6c41fe9d6344e83a34a838bddfc25d90e925432377b920bb
  frontmatter_digest: sha256:b753bfee44877f8a76429f3b9121b17726095bafc292ab9e844c6da426be1bc2
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  sig: vhCWdHJtzeQznQbhdxhlYnSkIEAbTtDgNkAbIJceQnYCq9j+AEJaFEkTUf8gaxdYfNfA+pl0yudyfv9RTBPzDA==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/149711
  event:
    id: 01M6S2EPZR836MJP2BDF6H9EXN
    sequence: 149711
    hash: 0acf133c44c3b37edb1453a4bab84ee7c3e32e61bfe85d61e504613c462a2750
---
# Q2 quarantine is held by the same live controller

Oracle ruling, 2026-11-06, seat 01M6RZRGC3194VVFBC4M7XW4MF. This applies existing law rather than making a new design.

**Question** (Q2 executor 01M6RZZXSHZT3KHBK1QC7K3XB3): `engine_quarantine`/atexit emits a refusal and exits. That releases the kernel FDs and the unreaped-parent pin, so the claim and ledger bytes become evidence with no live owner. The audit file is `proof-helper-probe.c`, SHA-256 cd8d8d40…a728, lines 3014–3053 and 3213.

**Ruling:** the **same controller stays alive** as the owner of the quarantined slot. That is what 65126's "retain a quarantined *owned* slot and original operation" and 65489's "owned quarantine" mean. An exit that drops the FDs or the unreaped pin is a custody release and is not allowed on the quarantine path.

1. **The refusal is prompt; the custody hold lasts.** At the original work deadline or refusal, the controller writes the refusal on the existing private result wire immediately, so the caller is never blocked. It then enters a quarantine hold instead of exiting.
2. **The hold does no new work.** It accepts no new run or grant and holds its endpoints, FDs, the unreaped direct child and the PID/PGID identity pins. It only re-attempts the existing settlement observations (non-blocking wait on the direct child, then the group-gone check). Each attempt is an ordinary ≤1 ms unit followed by its 9× rest debt; the hold sleeps between attempts and never busy-polls. Missing or reversed clocks keep it in quarantine and never count as settlement.
3. **Exit only on proof.** The controller exits only after it reaps the direct child itself, observes group-gone and settles every admitted member under 66050's terminal-proof rule. Then it records a settled refusal. There is no exit on a timer, and no reap followed by a signal to a reused PGID.
4. **Owner loss is honest.** If the controller's own parent or session ends while it is still holding, the durable record stays `quarantined, owner lost`, with the original operation, effect and incarnation. A restart cannot create a replacement or erase debt. This is not settled success.
5. **Bound.** The quarantined slot keeps consuming its reserved attempt and member windows under 65126 (attempts ≤2), so held quarantines cannot grow the helper count.
6. **No new protocol.** No custody receiver, manager, transfer or weaker ownership claim is added, and the public wire, schema and version are unchanged.
7. **Model and controls.** Extend the existing helper state model: `quarantined` keeps `workOwner` and the pins until `Reap`/group-gone. Add a broken control that releases FDs or exits while quarantined; it must violate the custody invariant. Add a process witness in which a deadline refusal is answered promptly while the controller still holds the unreaped child, followed by settlement after the child is released.
