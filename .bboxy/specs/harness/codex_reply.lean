-- Proofs for the theorems of codex_reply.den.
-- Checked by `make test-lean` against the statements `bboxy450-den lean` generates.
-- Origin: bboxy450-term, https://github.com/bboxy450/bboxy450-term
-- Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
import DenCodexReply
namespace DenCodexReply

/-- No step ever renders the transport envelope as the body. -/
theorem CODEXREPLY_001_proved : CODEXREPLY_001_statement := by
  intro σ reachable
  induction reachable with
  | initial => simp [CODEXREPLY_001, initial]
  | step _ taken held => cases taken <;> simp_all [CODEXREPLY_001]

/-- A reply is recorded only from `NoReply`, and every reply case leaves the
card out of `Pending`; nothing later returns it there. -/
theorem CODEXREPLY_002_proved : CODEXREPLY_002_statement := by
  intro σ reachable
  induction reachable with
  | initial => simp [CODEXREPLY_002, initial]
  | step _ taken held => cases taken <;> simp_all [CODEXREPLY_002] <;> grind

/-- Every step into `FailedCard` writes the message, and nothing leaves a
failed card's body. -/
theorem CODEXREPLY_003_proved : CODEXREPLY_003_statement := by
  intro σ reachable
  induction reachable with
  | initial => simp [CODEXREPLY_003, initial]
  | step _ taken held => cases taken <;> simp_all [CODEXREPLY_003] <;> grind

/-- A card starts spinning only before the turn ends, and ending the turn
stops a spinning card. -/
theorem CODEXREPLY_004_proved : CODEXREPLY_004_statement := by
  intro σ reachable
  induction reachable with
  | initial => simp [CODEXREPLY_004, initial]
  | step _ taken held => cases taken <;> simp_all [CODEXREPLY_004] <;> grind

end DenCodexReply
