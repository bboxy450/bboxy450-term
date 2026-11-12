-- Proofs for the theorems of outbox.den, for a message type of any size.
-- Checked by `make test-lean` against the statements `bboxy450-den lean` generates.
-- Origin: bboxy450-term, https://github.com/bboxy450/bboxy450-term
-- Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
import DenOutbox
namespace DenOutbox

/-- Inductive on its own: Submit refuses an acknowledged message, and Applied
removes from the outbox exactly the message it acknowledges. -/
theorem OUTBOX_001_proved : OUTBOX_001_statement := by
  intro Msg σ reachable
  induction reachable with
  | initial => intro m acked; exact acked.elim
  | step _ taken held =>
    cases taken <;> intro m <;> simp_all [OUTBOX_001] <;> grind

/-- Inductive on its own: Send needs the message unacknowledged, Applied drops
it from both sets, and Drop empties the flight. -/
theorem OUTBOX_002_proved : OUTBOX_002_statement := by
  intro Msg σ reachable
  induction reachable with
  | initial => intro m inflight; exact inflight.elim
  | step _ taken held =>
    cases taken <;> intro m <;> simp_all [OUTBOX_002] <;> grind

end DenOutbox
