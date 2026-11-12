-- Proof of REMOTE-DELIVERY-001 for the protocol! declaration in delivery.rs,
-- for a message type of any size. Checked by `make test-lean`.
-- Origin: bboxy450-term, https://github.com/bboxy450/bboxy450-term
-- Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
import YarnRemoteDelivery
namespace YarnRemoteDelivery

/-- The strengthening that makes it inductive: nothing was applied twice, and
every message ever applied is still recorded as applied. `Arrive` applies only
an unrecorded message, and `RemoteCrash` keeps the record (the runtime journals
it durably), so a replay after a crash is a duplicate, never a second apply. -/
theorem applied_is_remembered {Msg : Type} (σ : State Msg) (reachable : Reachable σ) :
    ¬ σ.doubled ∧ ∀ m, σ.ever m → σ.applied m := by
  induction reachable with
  | initial => exact ⟨fun doubled => doubled, fun _ ever => ever.elim⟩
  | step _ taken held =>
    cases taken <;> simp_all <;> grind

theorem applied_at_most_once_proved : applied_at_most_once_statement := by
  intro Msg σ reachable
  exact (applied_is_remembered σ reachable).1

end YarnRemoteDelivery
