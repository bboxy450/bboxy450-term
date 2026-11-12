-- Broken control: a proof of something weaker than the generated statement.
-- Origin: bboxy450-term, https://github.com/bboxy450/bboxy450-term
-- Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
import DenWeakened
namespace DenWeakened

theorem WEAK_001_proved : ∀ σ : State, Reachable σ → True := by
  intros; trivial

end DenWeakened
