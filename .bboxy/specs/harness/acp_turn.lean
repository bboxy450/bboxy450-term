-- Proofs for the theorems of acp_turn.den.
-- Checked by `make test-lean` against the statements `bboxy450-den lean` generates.
-- Origin: bboxy450-term, https://github.com/bboxy450/bboxy450-term
-- Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
import DenAcpTurn
namespace DenAcpTurn

/-- `opens` starts at 0 and only ToolStart, guarded by `opens < 2`, adds one. -/
theorem TYPE_OK_proved : TYPE_OK_statement := by
  intro σ reachable
  induction reachable with
  | initial => simp [TYPE_OK, initial]
  | step _ taken held => cases taken <;> simp_all [TYPE_OK] <;> omega

/-- Inductive on its own: the only step that sets `interrupted_asking` is the
watchdog, and its guard says nothing waits on the operator. -/
theorem ACPTURN_001_proved : ACPTURN_001_statement := by
  intro σ reachable
  induction reachable with
  | initial => simp [ACPTURN_001, initial]
  | step _ taken held => cases taken <;> simp_all [ACPTURN_001]

/-- The strengthening ACPTURN_002 needs: a card opens only from `NoTool`, and
nothing returns a call to `NoTool`, so it opens once. -/
theorem one_card (σ : State) (reachable : Reachable σ) :
    (σ.tool = Tool.NoTool → σ.opens = 0) ∧ σ.opens ≤ 1 := by
  induction reachable with
  | initial => simp [initial]
  | step _ taken held => cases taken <;> simp_all <;> omega

theorem ACPTURN_002_proved : ACPTURN_002_statement := by
  intro σ reachable
  exact (one_card σ reachable).2

/-- Inductive on its own: parking requires that steering cannot deliver into a
running turn, negotiation delivers what is parked in one, and a turn that
starts from parked input takes it as its prompt. -/
theorem ACPTURN_003_proved : ACPTURN_003_statement := by
  intro σ reachable
  induction reachable with
  | initial => simp [ACPTURN_003, initial]
  | step _ taken held => cases taken <;> simp_all [ACPTURN_003] <;> grind

end DenAcpTurn
