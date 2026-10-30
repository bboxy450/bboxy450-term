#!/bin/sh
# Local-first invariant (.bboxy/invariants/local-first.md): until the cloud phase
# opens, the mux depends on no remote transport or cloud SDK. Operator-authorized
# launch cut 2 invokes local ssh/mosh/et clients; cloud providers remain separate.
set -eu
cd "$(dirname "$0")/.."
hits=$(cargo tree -p bboxy450-term-mux -e normal --locked | grep -iE 'quinn|russh|ssh2|aws-sdk|reqwest|tonic|hyper-util|rustls' || true)
if [ -n "$hits" ]; then printf '%s\n' "$hits"; echo "local-first: VIOLATED"; exit 1; fi
echo "local-first: held"
