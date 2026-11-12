-- Broken control: the statement is assumed as an axiom instead of proved.
-- Origin: bboxy450-term, https://github.com/bboxy450/bboxy450-term
-- Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
import DenAssumed
namespace DenAssumed

axiom trust_me : ASSUMED_001_statement

theorem ASSUMED_001_proved : ASSUMED_001_statement := trust_me

end DenAssumed
