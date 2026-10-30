# coverage: full
# bboxy450-term — https://github.com/bboxy450/bboxy450-term
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
. "$(dirname "$0")/lib.sh"
set -e
perf="$(dirname "$BBT")/bboxy450-perf-p1"
[ -x "$perf" ] || fail 'P1 fixture_unavailable: run make build-perf-p1 before the night'
"$perf" ctl debug performance > "$E2E_WORK/p1.jsonl" || fail 'P1 private headless fixture failed'
python3 -I - "$E2E_WORK/p1.jsonl" <<'PY'
import json,sys
r=json.load(open(sys.argv[1]))['data']
assert r['rows']==1000 and r['watched']==50 and r['configured_hz']==120,r
assert r['idle_hitches']==r['paging_hitches']==r['idle_frames']==0,r
assert r['paging_frames']==8 and r['gpu_completed']==r['submitted_frames'],r
assert 0<r['retained_heap_bytes']<=r['heap_budget_bytes']==200_000_000,r
assert r['pages_fetched']>=50 and r['passed'],r
print('P1: 1000 sidebar rows, 50 actual History owners; native HUD/GPU, zero idle/paging hitches; bounded retained heap',r)
PY

"$perf" ctl debug performance broken > "$E2E_WORK/p1-broken.jsonl" || fail 'P1 broken control did not complete'
python3 -I - "$E2E_WORK/p1-broken.jsonl" <<'PYCONTROL'
import json,sys
r=json.load(open(sys.argv[1]))['data']
assert r['paging_hitches']>0 and r['passed'] is False,r
print('P1: real delayed-layout broken control rejected',r['paging_hitches'])
PYCONTROL
