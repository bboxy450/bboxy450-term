-- Broken control. counted.den is sound for its two members, and false for three:
-- no proof exists, and `sorry` must not count as one.
-- Origin: bboxy450-term, https://github.com/bboxy450/bboxy450-term
-- Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
import DenCounted
namespace DenCounted

theorem TYPE_OK_proved : TYPE_OK_statement := by
  intro D σ reachable
  induction reachable with
  | initial => simp [TYPE_OK, initial]
  | step _ taken held => cases taken <;> simp_all [TYPE_OK] <;> omega

theorem COUNTED_001_proved : COUNTED_001_statement := by
  sorry

end DenCounted
