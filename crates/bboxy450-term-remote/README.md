# bboxy450-term-remote

The remote supervisor link for bboxy450-term. A swarm can be placed on a cloud
VM or SSH host. One remote supervisor serves the whole swarm and hosts only
bboxy450-harness seats over ACP. The link is one protocol: length-prefixed JSON
frames carrying Log IR v1 envelopes byte-for-byte.

The remote host needs native builds of `bboxy450-term-remote` and
`bboxy450-harness`, plus `/usr/bin/nc` for the harness's MCP socket bridge
(`nmap-ncat` on Amazon Linux).

Native agent commands run on the SSH host. Forwarded MCP servers still run
locally, including BBoxy's Python command runtime. Use native agent commands
for remote files and processes. This bridge does not provide managed remote
workspaces.

It has these parts:

- the frame codec (`frame`);
- the exactly-once delivery model (`delivery`, a Yarn `protocol!` checked by
  the native Rust TLC and by Java TLC);
- its runtime (`link`: `Outbox`, `Inbox`, `FileJournal`);
- ordered event resume (`resume_model`, `resume`: `SessionLog`, `Cursor`), which resets the view on a new epoch;
- the relay (`serve`, binary `bboxy450-term-remote serve --state DIR`). Each session is one `bboxy450-harness acp` process: ops are ACP messages written exactly once, and agent output is logged for replay.

A witness test drives the model and the runtime through the same steps.

The interface is `.bboxy/interfaces/remote-supervisor-link.md`. Project policy:
https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
