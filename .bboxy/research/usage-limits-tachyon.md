---
id: "1Q3W1615JQCSDGS5P57E4VTS61"
type: "research"
number: 67
title: "Usage limits — how `tachyon` reads Claude and Codex account limits"
created: "2026-10-21T18:54:36Z"
updated: "2026-10-24T00:10:38Z"
---
# Usage limits — how `tachyon` reads Claude and Codex account limits

Back to [[roadmap]] · [[vision]].

Source: local clone `~/mydev/tachyon` (remote `github.com/Gonzih/tachyon`, HEAD `eab6d72` 2026-09-15). Swift, macOS. Paths below are relative to that repo. No token values recorded here; field names only.

## Claude Code

- Endpoint: `GET https://api.anthropic.com/api/oauth/usage` — `Sources/Tachyon/Providers/ClaudeProvider.swift:31`. Profile endpoint `/api/oauth/profile` at `:32` (account fingerprint only).
- Headers: `Authorization: Bearer <token>`, `anthropic-beta: oauth-2025-04-20` (required), `Content-Type: application/json`, `User-Agent` — `ClaudeProvider.swift:348-355`.
- Credential priority (`ClaudeProvider.swift:575-626`):
  1. env `CLAUDE_CODE_OAUTH_TOKEN` — `:587`.
  2. macOS Keychain generic password, read via `/usr/bin/security find-generic-password -s <service> -w` — `:99-101`. Service = `"Claude Code-credentials"` (`:33`), suffixed `-<first 4 bytes of sha256(config dir)>` when a profile-specific config dir is set — `:543-566`.
  3. File `$CLAUDE_CONFIG_DIR/.credentials.json` (default `~/.claude/.credentials.json`) — `:549,563-564`.
- Payload shape for 2 and 3 is identical: `claudeAiOauth.accessToken`, `claudeAiOauth.expiresAt` (ms epoch), `subscriptionType`, `rateLimitTier` — `ClaudeProvider.swift:677-690`.
- Response decode (`ClaudeProvider.swift:423-470`): `five_hour.utilization` + `five_hour.resets_at`; `limits[]` rows with `kind` (`session`, `weekly_all`, …), `percent`, `resets_at`, `is_active`, `scope`; `seven_day.utilization` used only when no `weekly_all` row.
- Rule: never refresh the harness's OAuth token — `AGENTS.md:178`.

## Codex CLI

- Endpoint (Stream A, primary): `GET https://chatgpt.com/backend-api/wham/usage` — `Sources/Tachyon/Providers/CodexProvider.swift:23`. Non-`chatgpt.com` base URLs map to `/api/codex/usage` — `:570-583`.
- Headers: `Authorization: Bearer <access_token>`, plus `ChatGPT-Account-Id` only when non-empty — `CodexProvider.swift:559-569`.
- Credential: `$CODEX_HOME/auth.json` (default `~/.codex/auth.json`) — `:55-61`. Fields read at `:683-704`: `tokens.access_token`, `tokens.account_id`, `tokens.id_token` (JWT; claim `https://api.openai.com/auth` → `chatgpt_plan_type`, fallback `chatgpt_account_id`).
- Response decode (`CodexProvider.swift:190-256`): `rate_limit.primary_window` / `secondary_window` `{used_percent, limit_window_seconds, reset_at | reset_after_seconds}`; `additional_rate_limits[]` per model; `plan_type`.
- Fallback (Stream B): isolated `codex app-server` JSON-RPC probe with an empty temp `CODEX_HOME` and no refresh token — `Sources/Tachyon/Providers/CodexAppServerProbe.swift`, described `kb/RESEARCH.md:70,73`. Signed-out fallback: tail `$CODEX_HOME/sessions/**/rollout-*.jsonl` `token_count` events — `CodexProvider.swift:397-410`, `kb/RESEARCH.md:71`.

## Transfer notes for bboxy450-term

- Both are one authenticated GET with a bearer read from a local file; no OAuth dance, no refresh. Poll 120s (Claude) / 60s (Codex) per `README.md:71-73`.
- `kb/RESEARCH.md:11-12,69` is the compressed cross-tool survey if the endpoints shift.

**Gap:** whether `api.anthropic.com/api/oauth/usage` still accepts `anthropic-beta: oauth-2025-04-20` as of 2026-10 — tachyon last touched 2026-09-15; not re-verified live.
**Gap:** Linux Claude Code stores only the file path (no Keychain); not checked in tachyon, which is macOS-only.
