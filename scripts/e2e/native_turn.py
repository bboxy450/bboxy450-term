"""Wait through native Codex startup without mistaking its initial prompt for a turn."""
import json
import time

import native_checks as native
from ui import require


def wait_codex_turn(app, seat, marker, prefix):
    deadline = time.monotonic() + 30
    approved = False
    while True:
        screen = app.ctl('seat', 'screen', seat)
        text = json.dumps(screen)
        if 'Hooks need review' in text:
            if not approved:
                # The existing recipe checks installed hook count and repository
                # hook absence before journaled input into this seat only.
                approved = native.approve_codex_hooks(seat, marker=marker, prefix=prefix)
        elif marker in text:
            return screen
        require(time.monotonic() < deadline, f'Codex turn {marker}: {text[-1500:]}')
        time.sleep(.05)
