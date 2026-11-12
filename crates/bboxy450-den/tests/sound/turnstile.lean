-- Proofs for turnstile.den: a named enum and a bounded integer.
-- Origin: bboxy450-term, https://github.com/bboxy450/bboxy450-term
-- Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
import DenTurnstile
namespace DenTurnstile

/-- The Lean state holds an unbounded `Int`; this is what keeps it Den's machine. -/
theorem TYPE_OK_proved : TYPE_OK_statement := by
  intro σ reachable
  induction reachable with
  | initial => simp [TYPE_OK, initial]
  | step _ taken held => cases taken <;> simp_all [TYPE_OK] <;> omega

theorem TURN_001_proved : TURN_001_statement := by
  intro σ reachable
  have range := TYPE_OK_proved σ reachable
  induction reachable with
  | initial => simp [TURN_001, initial]
  | step before taken held =>
    have earlier := TYPE_OK_proved _ before
    cases taken <;> simp_all [TURN_001, TYPE_OK] <;> omega

end DenTurnstile
