# coverage: partial; missing: injectable ring-gap fixture; displayed gap row and both surviving sides
# The existing HistoryRecord::Gap unit fixture is not a running owned ring fixture.
# Never mutate/open a live Ring from a night script to manufacture a passing gap.
. "$(dirname "$0")/lib.sh"
fail 'S2 fixture_unavailable: no ctl ring-gap fixture; GUI gap placement remains unproved'
