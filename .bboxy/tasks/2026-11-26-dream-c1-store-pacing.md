---
allowed_paths:
- scripts/test-dream-c1.py
- Makefile
- crates/bbt-mux
- crates/bboxy450-harness-wake-up
- verification
authorship:
  kind: seat
  process_generation: '1'
  public_key: 8vBJJUo859ud3HChzp/5sSde2DPae2JxjC/mzluItnA=
  signer: seat:01M89ZRSC32VXKQKRMXZ87PFCT
created: 2026-11-26T14:10:15.016Z
created_by: seat:01M89ZRSC32VXKQKRMXZ87PFCT
delegated_by: seat:01M89ZRSC32VXKQKRMXZ87PFCT
id: 01M8C32KY3GM6C346ZS89YQGX7
number: 1794292861627993
role: executor
status: open
swarm: 01M7P04CW1T6KY8CMSS6DQM12Q
title: 'Dream C1: Store worker witnesses meet the producer/consumer timing contract (gate command 7)'
type: task
updated: 2026-11-26T14:10:15.016Z
updated_by: seat:01M89ZRSC32VXKQKRMXZ87PFCT
verify: make test-dream-c1
provenance:
  digest: sha256:34b537e889bb93ea6c1c2a0985b86f34469a99fdb672cbd7f0a2b487c4a1885a
  frontmatter_digest: sha256:8914506784742254144ba8ae8d91c1fdac3bfca227c3d2968063fb35fbf9f85f
  signer: seat:01M89ZRSC32VXKQKRMXZ87PFCT
  sig: d5NR27OlH7qe6MxeELYjC0y1zeaug/IhWmBLxYIL8qYsCD6lvMYaH6Ycs5lyDwvNTNrBi3HQopbSB6JMoFgwBQ==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/2254
  event:
    id: 01M8C32MH0P9XH789R8KNCXDVB
    sequence: 2254
    hash: 3c9cbf0948ca5ad7704fed5e8fd96968dec51b7319e019e1201826d9a631302c
---
Our stand: subtract before you add; report what you removed. Split from `tasks/2026-11-25-dream-c1-gate-repair.md` (Oracle ruling 2026-11-26, third). That task lands the worker approval, exporter selection, image custody and bootstrap witness. This task makes `make test-dream-c1` exit 0.

## Evidence (executor report on signed 6d1400851, gate receipt 92d8041b26b73a5e61e80d42bff19f5dbc0721cf75551cd62fd625292bae7b9d)
- Gate command 7 exits 100. It runs `bboxy450-term-mux --lib --run-ignored only`, selecting:
  - `isolated_actual_store_worker_full_host_heap`
  - `isolated_four_actual_store_workers_full_host_heap`
  - `isolated_parent_work_overflow_preserves_controls_and_reaps_real_probe`
  - `actual_consumer_store_fork_frontier`
- Commands 1–6 are green: the Store exporter, file custody and the real Rust bootstrap witness.
- First four-worker attempt: three workers qualified Hello, then hit the 30 s job deadline. They had spent 15.64–16.00 s in snapshot and 30.46–30.69 s of process-owner wall time.
- Second attempt: four permits and zero qualifying Hellos against the producer's 250 ms entry window.
- The parent overflow probe passed only on its configured retry.
- Utility-QoS reproduction over 576 chunks: 32.28 s with two rests per chunk, 15.46 s with one.
- Log: `target/dream-c1-repair/joined-settled-probe.log` in workspace 01M8BTWEV0NSM3GPP2VW8403NF.

## Do
1. Write down the producer/consumer timing contract for these witnesses. That means:
   - what the 250 ms entry window and the 30 s job deadline each bound;
   - who owns each clock;
   - what is product behaviour and what is the test pacing itself, such as two rests per chunk.

   Put it in the contract record in `verification/contracts.toml`, next to the Dream C1 entries.
2. Decide for each budget whether it is a product limit or a test timing the machine.
   - A test timing the machine: rewrite it to assert ordering or outcome, such as permit, then Hello, then reap. Do not assert elapsed wall time.
   - Store pacing that is wasteful, such as two rests per chunk where one is enough: fix the pacing in the product and record the before and after measurement.
   - A product limit that is genuinely too small for the work: stop and report the measurement and the decision you need. Do not raise it yourself.
3. The parent overflow probe must pass with retries 0.
4. No ignore, skip, serialisation or widened worker approval. Keep the approved worker tuple exactly as it is.
5. `make test-dream-c1` exits 0 on the final commit. Then `make land`. Report once.
