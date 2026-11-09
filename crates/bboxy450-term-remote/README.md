# bboxy450-term-remote

The remote supervisor link for bboxy450-term. A swarm can be placed on a cloud
VM or SSH host. One remote supervisor serves the whole swarm and hosts only
bboxy450-harness seats over ACP. The link is one protocol: length-prefixed JSON
frames carrying Log IR v1 envelopes byte-for-byte.

This first cut has three parts:

- the frame codec (`frame`);
- the exactly-once delivery model (`delivery`, a Yarn `protocol!` checked by
  the native Rust TLC and by Java TLC);
- its runtime (`link`: `Outbox`, `Inbox`, `FileJournal`).

A witness test drives the model and the runtime through the same steps.

The interface is `.bboxy/interfaces/remote-supervisor-link.md`. Project policy:
https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
