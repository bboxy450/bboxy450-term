---
allowed_paths:
- scripts/cache-env.sh
- scripts/test-landing.py
- Makefile
authorship:
  kind: seat
  process_generation: '1'
  public_key: 8ZJ1pn6ymQlhdn20cJBq2Rono7S6d/fLcgc3FK45CmU=
  signer: seat:01M8764DX3P3XKP4DF8R2DANYR
created: 2026-11-24T17:09:30.840Z
created_by: seat:01M8764DX3P3XKP4DF8R2DANYR
delegated_by: seat:01M8764DX3P3XKP4DF8R2DANYR
id: 01M878HDJ9H49E3D3WNB42GXX4
number: 1794292861627913
role: executor
status: open
swarm: 01M7P04CW1T6KY8CMSS6DQM12Q
title: 'Keep Spotlight out of build dirs: cache-env.sh writes .metadata_never_index into target/ and kache dirs'
type: task
updated: 2026-11-24T17:09:30.840Z
updated_by: seat:01M8764DX3P3XKP4DF8R2DANYR
verify: make test-landing
provenance:
  digest: sha256:8b192e19d58bb23dd93c89964c3978f54004b9bda8e94955f4d654cc32b91fef
  frontmatter_digest: sha256:c362cb29de0fc24b681ff911cf2379475a541fd7954123b967b5c163c8d2bd86
  signer: seat:01M8764DX3P3XKP4DF8R2DANYR
  sig: KAkF7LUwdvHqyo3rH7vaH242ksEDF3Udmb2fa7T0JTDH+DGy/ORems4ViWugdbz5Nf0DvVrU8XNirizHz4oUAg==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/1845
  event:
    id: 01M878HE6SW2FCZQTXB678CRS2
    sequence: 1845
    hash: 8defa8d9bfda0c5943c64ce51273e700c7ed9a81111d85eac71f9dbd60c038d5
---
Our stand: subtract before you add; report what you removed. Read the legacy record(s) named below for the original brief and evidence.

Legacy: `tasks/2026-11-06-build-dirs-out-of-spotlight-fsevents.md`.

## Evidence
No `.metadata_never_index`, `mdutil` or Spotlight handling in `scripts/`, `Makefile` or `LAND.md` on origin/main 911587f2f.

## Do
Have `scripts/cache-env.sh` create `.metadata_never_index` in the selected target dir, the verification evidence dir and the kache cache dir (idempotent, cheap). Assert it in `scripts/test-landing.py`. `make land`.
