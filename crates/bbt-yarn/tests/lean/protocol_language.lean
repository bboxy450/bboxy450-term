-- Proofs for the ProtocolLanguage declaration in tests/protocol_runtime.rs.
-- Checked by `make test-lean`.
-- Origin: bboxy450-term, https://github.com/bboxy450/bboxy450-term
-- Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
import YarnProtocolLanguage
namespace YarnProtocolLanguage

theorem complementary_proved : complementary_statement := by
  intro σ reachable
  induction reachable with
  | initial => simp [complementary, initial]
  | step _ taken held => cases taken <;> simp_all [complementary] <;> grind

theorem set_algebra_proved : set_algebra_statement := by
  intro σ reachable
  simp [set_algebra]
  grind

theorem option_universe_proved : option_universe_statement := by
  intro σ reachable
  simp only [option_universe]
  cases selected : σ.selected with
  | none => simp
  | some mode => cases mode <;> simp

end YarnProtocolLanguage
