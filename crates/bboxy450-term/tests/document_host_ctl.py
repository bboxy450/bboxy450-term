#!/usr/bin/env python3
"""Unattended native document/terminal smoke; every verdict is an assertion."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import struct
import subprocess
import time

p = argparse.ArgumentParser()
p.add_argument('--bin', type=Path, required=True)
p.add_argument('--source-root', type=Path, required=True)
p.add_argument('--out', type=Path, required=True)
a = p.parse_args()
binary, source, out = a.bin.resolve(), a.source_root.resolve(), a.out.resolve()
out.mkdir(parents=True, exist_ok=False)
project, runtime, custody = (out / n for n in ['project', 'runtime', 'custody'])
project.mkdir()
tracked = subprocess.check_output(['git', '-C', str(source), 'ls-files', '-z', '--', '.bboxy']).split(b'\0')
for name in filter(None, tracked):
    rel = Path(os.fsdecode(name))
    original = source / rel
    if rel.suffix == '.md' and original.is_file() and not original.is_symlink():
        dest = project / rel
        dest.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(original, dest)
zsh = out / 'zsh'
zsh.mkdir()
(zsh / '.zshrc').write_text('PROMPT="doc-test> "\n')
config = out / 'config.toml'
config.write_text('[appearance]\ntheme="dark"\ndensity="cozy"\nrim=4\n[notifications]\nenabled=false\n')
env = os.environ.copy()
env['BBOXY_TEST_CUSTODY'] = 'fixture:' + str(custody)
env['ZDOTDIR'] = str(zsh)
ready, png, log_path = out / 'ready.png', out / 'document.png', out / 'app.log'
argv = [str(binary), '--run-dir', str(runtime), '--ctl', '--document', '#44',
        '--settings-config', str(config), '--capture', str(ready), '--quit-after', '60']
proof = {'source': subprocess.check_output(['git', '-C', str(source), 'rev-parse', 'HEAD'], text=True).strip(),
         'scope': 'Current #44 native document, concurrent terminal IO and ctl capture; focus/scroll/error semantics are asserted by the Rust integration tests',
         'custody': 'fresh generated debug fixture keys in the private run; never login Keychain', 'commands': []}
proc = None
failure = None
try:
    with log_path.open('w') as log:
        proc = subprocess.Popen(argv, cwd=project, env=env, stdout=log, stderr=subprocess.STDOUT)
        proof['owned_pid'] = proc.pid
        def ctl(*words, allow_startup=False):
            result = subprocess.run([str(binary), 'ctl', *words, '--run-dir', str(runtime),
                                     '--as', 'document-host-test'], cwd=project, env=env,
                                    text=True, capture_output=True, timeout=20)
            rows = [json.loads(line) for line in result.stdout.splitlines() if line.strip()]
            if allow_startup and result.returncode == 4:
                return None
            assert result.returncode == 0 and len(rows) == 1 and rows[0].get('ok'), f'ctl {words[:2]} exit {result.returncode}'
            proof['commands'].append({'command': list(words[:2]), 'exit': result.returncode, 'op': rows[0].get('op')})
            return rows[0]['data']
        deadline = time.monotonic() + 40
        while ctl('seat', 'ls', allow_startup=True) is None:
            assert time.monotonic() < deadline and proc.poll() is None, 'private ctl endpoint never became ready'
            time.sleep(0.2)
        spawned = ctl('seat', 'spawn', 'zsh', '--name', 'DocsFixture')
        seat = spawned['seat']
        marker = '__DOCUMENT_CTL_IO_LIVE__'
        ctl('seat', 'type', seat, "printf '" + marker + "\\n'\r")
        ctl('seat', 'wait', seat, '--text', '(?m)^' + marker + '[ \\t]*$', '--timeout', '10000')
        while not ready.is_file():
            assert time.monotonic() < deadline and proc.poll() is None, 'document never reached its capture-ready state'
            time.sleep(0.2)
        text = log_path.read_text()
        assert 'fixture custody (debug)' in text and 'document #44: "loaded"' in text, 'document/custody readiness was not proven'
        assert 'document #44: "refused"' not in text, 'document read was refused'
        frame = ctl('seat', 'capture', 'window', str(png))
        assert Path(frame['path']).resolve() == png and png.is_file()
        data = png.read_bytes()
        assert data[:8] == b'\x89PNG\r\n\x1a\n'
        width, height = struct.unpack('>II', data[16:24])
        assert width >= 800 and height >= 600 and frame['frame_presented_at'] is not None
        screen = ctl('seat', 'screen', seat)
        assert any(re.fullmatch(marker + r'[ \t]*', row) for row in screen['rows']), 'terminal output lost during document reads'
        proof.update(seat=seat, frame=frame, image={'path': str(png), 'width': width, 'height': height,
                     'sha256': hashlib.sha256(data).hexdigest()}, document_loaded=True, terminal_io_live=True)
        assert proc.wait(timeout=75) == 0, 'app shutdown failed'
        assert 'children reaped after orderly shutdown' in log_path.read_text(), 'children were not reaped'
except Exception as error:
    failure = f'{type(error).__name__}: {error}'
finally:
    if proc is not None and proc.poll() is None:
        try:
            proc.wait(timeout=75)
        except subprocess.TimeoutExpired:
            proc.terminate()
            try:
                proc.wait(timeout=10)
            except subprocess.TimeoutExpired:
                proc.kill()
                proc.wait()
            failure = failure or 'owned verification process exceeded its shutdown bound'
    proof['process_exit'] = None if proc is None else proc.returncode
    for directory in [custody, runtime, project, zsh]:
        if directory.exists():
            shutil.rmtree(directory)
    proof.update(pass_=failure is None, failure=failure, private_account_retired=True)
    (out / 'verdict.json').write_text(json.dumps(proof, indent=2) + '\n')
print(json.dumps(proof, indent=2))
raise SystemExit(0 if failure is None else 1)
