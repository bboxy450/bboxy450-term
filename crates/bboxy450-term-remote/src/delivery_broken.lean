-- Broken control for BrokenRemoteDelivery: a remote that forgets what it applied
-- applies twice, so REMOTE-DELIVERY-001 has no proof and `sorry` must not count.
-- Origin: bboxy450-term, https://github.com/bboxy450/bboxy450-term
-- Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
import YarnBrokenRemoteDelivery
namespace YarnBrokenRemoteDelivery

theorem applied_at_most_once_proved : applied_at_most_once_statement := by
  sorry

end YarnBrokenRemoteDelivery
