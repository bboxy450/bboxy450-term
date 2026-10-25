"""Deterministic rendered-pixel assertions for R1, R3 and R5."""
import hashlib
import colorsys
import json
import math
import os
from pathlib import Path
import struct
import subprocess
import sys
import uuid
import zlib
import tomllib


def png(path):
    """The app's non-interlaced 8-bit RGBA captures, with all PNG row filters."""
    data = Path(path).read_bytes()
    assert data[:8] == b'\x89PNG\r\n\x1a\n', 'capture is not PNG'
    at = 8
    compressed = []
    width = height = None
    while at < len(data):
        length = struct.unpack_from('>I', data, at)[0]
        kind = data[at + 4:at + 8]
        payload = data[at + 8:at + 8 + length]
        assert len(payload) == length
        if kind == b'IHDR':
            width, height, depth, colour, comp, filt, interlace = struct.unpack('>IIBBBBB', payload)
            assert depth == 8 and colour == 6 and comp == filt == interlace == 0, 'expected app RGBA8 PNG'
        elif kind == b'IDAT':
            compressed.append(payload)
        elif kind == b'IEND':
            break
        at += length + 12
    assert width and height
    scan = zlib.decompress(b''.join(compressed))
    stride = width * 4
    assert len(scan) == height * (stride + 1), 'truncated PNG rows'
    pixels = bytearray()
    prior = bytearray(stride)
    for y in range(height):
        offset = y * (stride + 1)
        mode = scan[offset]
        row = bytearray(scan[offset + 1:offset + stride + 1])
        assert mode <= 4, 'unknown PNG row filter'
        if mode:
            for i in range(stride):
                left = row[i - 4] if i >= 4 else 0
                up = prior[i]
                corner = prior[i - 4] if i >= 4 else 0
                if mode == 1:
                    prediction = left
                elif mode == 2:
                    prediction = up
                elif mode == 3:
                    prediction = (left + up) // 2
                else:
                    p = left + up - corner
                    distances = (abs(p - left), abs(p - up), abs(p - corner))
                    prediction = (left, up, corner)[distances.index(min(distances))]
                row[i] = (row[i] + prediction) & 255
        pixels.extend(row)
        prior = row
    return width, height, pixels


def region(image, rect):
    w, h, pixels = image
    x0, y0, x1, y1 = map(int, rect)
    assert 0 <= x0 < x1 <= w and 0 <= y0 < y1 <= h, (rect, w, h)
    return b''.join(pixels[(y * w + x0) * 4:(y * w + x1) * 4] for y in range(y0, y1))


def changed(a, b, rect):
    assert a[:2] == b[:2]
    x, y = region(a, rect), region(b, rect)
    return sum(x[i:i + 4] != y[i:i + 4] for i in range(0, len(x), 4))


def geometry(frame):
    return [{k: pane[k] for k in ('seat', 'rect', 'cols', 'rows', 'focused')} for pane in frame['panes']]


def nodes(frame, kind):
    def walk(node):
        if node['kind'] == kind:
            yield node
        for child in node.get('children', []):
            yield from walk(child)
    return [node for tree in frame['tree'] for node in walk(tree)]


def halo_gap(frame):
    pane = nodes(frame, 'named:pane')[0]['rect']
    new_swarm = nodes(frame, 'named:new-swarm')[0]['rect']
    dock = nodes(frame, 'named:conversation/rows')[0]['rect']
    return pane, (pane[0] - 32, new_swarm[1] + new_swarm[3] + 16, pane[0] - 8, dock[1] - 16)


class Run:
    def __init__(self, promise):
        self.promise = promise
        self.binary = os.environ['BBT']
        self.work = Path(os.environ['E2E_WORK']).resolve()
        self.out = self.work / (promise.lower() + '-clock-' + uuid.uuid4().hex[:8])
        self.out.mkdir(exist_ok=False)
        self.calls = []
        self.frozen = False
        self.last_clock = None
        self.kit = Path(os.environ.get('E2E_KIT_ROOT', Path(__file__).resolve().parents[2]))
        self.tokens = tomllib.loads((self.kit / 'design/tokens.toml').read_text())
        self.style = json.loads((self.kit / 'design/style.json').read_text())

    def ctl(self, *args):
        args = tuple(map(str, args))
        result = subprocess.run([self.binary, 'ctl', *args], cwd=self.work,
                                capture_output=True, text=True, timeout=45)
        rows = [json.loads(line) for line in result.stdout.splitlines() if line.strip()]
        self.calls.append({'args': args, 'exit': result.returncode, 'rows': rows, 'stderr': result.stderr})
        assert result.returncode == 0 and rows and rows[-1].get('ok') is True, self.calls[-1]
        return rows[-1]['data']

    def clock(self, verb, ms=None):
        args = ('ui', 'clock', verb) + (() if ms is None else (str(ms),))
        frame = self.ctl(*args)
        state = frame['clock']
        assert isinstance(frame['frame_seq'], int) and frame['frame_seq'] > 0
        assert state['frozen'] is (verb != 'resume')
        if self.last_clock is not None:
            assert state['revision'] > self.last_clock['clock']['revision']
            assert frame['frame_seq'] > self.last_clock['frame_seq']
            if verb == 'advance':
                assert state['time_ms'] == self.last_clock['clock']['time_ms'] + ms
        self.last_clock = frame
        self.frozen = state['frozen']
        return frame

    def capture(self, name):
        path = self.out / (name + '.png')
        receipt = self.ctl('seat', 'capture', 'window', path)
        assert receipt['frame_seq'] >= self.last_clock['frame_seq'] and receipt['stale'] is False
        assert receipt['rendered_at_ms'] > 0
        return png(path), receipt

    def shell(self, name, tab=True, stable=True):
        args = ('--place', 'tab') if tab else ()
        seat = self.ctl('seat', 'spawn', 'zsh', '--cwd', self.work, '--name', name, *args)['seat']
        if stable:
            marker = self.promise + '-READY'
            self.line(seat, "printf '\\033[?25l\\n" + marker + "\\n'; exec cat")
            self.ctl('seat', 'wait', seat, '--text', '(?m)^' + marker + '$', '--timeout', '20000')
        return seat

    def line(self, seat, line):
        self.ctl('seat', 'type', seat, line)
        self.ctl('seat', 'key', seat, 'enter')

    def start(self):
        status = self.ctl('app', 'status')
        assert status['running'] and status['gui_ready'] and status['custody_backend'] == 'file'
        assert status['unattended'] and status['windows'] == 0, 'night owns a headless test account'
        self.clock('freeze')  # The admitted owner refuses a non-test account.

    def finish(self, verdict=None):
        cleanup_error = None
        if self.frozen:
            try:
                self.clock('resume')
            except Exception as error:
                cleanup_error = error
                self.calls.append({'cleanup_error': str(error)})
        (self.out / 'commands.json').write_text(json.dumps(self.calls, indent=2))
        manifest = {'binary_sha256': hashlib.sha256(Path(self.binary).read_bytes()).hexdigest(),
                    'driver_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                    'kit_tokens_sha256': hashlib.sha256((self.kit / 'design/tokens.toml').read_bytes()).hexdigest(),
                    'kit_style_sha256': hashlib.sha256((self.kit / 'design/style.json').read_bytes()).hexdigest(),
                    'captures': {path.name: {'sha256': hashlib.sha256(path.read_bytes()).hexdigest(),
                                             'bytes': path.stat().st_size} for path in self.out.glob('*.png')}}
        (self.out / 'manifest.json').write_text(json.dumps(manifest, indent=2))
        if verdict is not None:
            (self.out / 'verdict.json').write_text(json.dumps(verdict, indent=2))
            if cleanup_error is not None:
                raise cleanup_error


def r3(run):
    seats = [run.shell('R3-first'), run.shell('R3-second')]
    run.ctl('ui', 'focus', seats[1])
    run.clock('advance', 900)
    run.ctl('ui', 'focus', seats[0])
    origin = run.clock('freeze')
    first, c0 = run.capture('origin')
    repeat, cr = run.capture('frozen-repeat')
    assert first == repeat, 'a frozen animation changed between captures'
    pane, gap = halo_gap(origin)
    stepped = run.clock('advance', 450)
    peak, cp = run.capture('peak-450')
    assert geometry(stepped) == geometry(origin), 'focus swell moved the solved pane or grid'
    count = changed(first, peak, gap)
    assert count > 0, 'no swell outside the resting rim at 450ms'
    content = next(p['rect'] for p in stepped['panes'] if p['seat'] == seats[0])
    content_box = (content[0], content[1], content[0] + content[2], content[1] + content[3])
    assert region(first, content_box) == region(peak, content_box), 'swell/ripple covered terminal content'
    far = (pane[0] - 64, gap[1], pane[0] - 44, gap[3])
    assert changed(first, peak, far) == 0, 'swell reached beyond 40px plus its rim half-width'
    distances = []
    for x in range(int(pane[0] - 44), int(pane[0] - 8)):
        if changed(first, peak, (x, gap[1], x + 1, gap[3])):
            distances.append(pane[0] - (x + .5))
    assert distances and 39 <= max(distances) <= 43, 'peak does not reach the documented 40px'
    settled = run.clock('advance', 450)
    after, ca = run.capture('settled-900')
    assert geometry(settled) == geometry(origin)
    assert changed(first, after, gap) == 0, 'single focus swell did not settle after 900ms'
    assert region(first, content_box) == region(after, content_box), 'terminal content moved after settling'
    return {'promise': 'R3', 'origin': origin, 'peak': stepped, 'settled': settled,
            'halo_pixels': count, 'visible_reach_px': max(distances),
            'content_identical': True, 'frozen_identical': True,
            'captures': [c0, cr, cp, ca]}


def r5(run):
    seat = run.shell('R5-waiting', stable=False)
    run.ctl('ui', 'focus', seat)
    run.clock('advance', 900)
    command = r'''printf '\033[?25l'; for r5_episode in 1 2; do printf '\033]7501;state=blocked:kind=question\007'; printf '\nR5-WAIT-%s\n' "$r5_episode"; read r5_answer; printf '\033]7501;state=clear\007'; printf '\nR5-CLEAR-%s\n' "$r5_episode"; if [ "$r5_episode" = 1 ]; then read r5_again; fi; done; exec cat'''
    run.line(seat, command)
    run.ctl('seat', 'wait', seat, '--state', 'waiting', '--timeout', '20000')
    run.ctl('seat', 'wait', seat, '--text', '(?m)^R5-WAIT-1$', '--timeout', '20000')
    origin = run.clock('freeze')
    first, c0 = run.capture('waiting-origin')
    pane, gap = halo_gap(origin)
    row = nodes(origin, 'named:swarm0/body/tab0/seat0/row')[0]['rect']
    # Left of this row, far from the pane's own halo: the pane cannot make
    # a missing row animation pass this assertion.
    row_gap = (max(0, row[0] - 9), row[1] + 4, row[0] - 3, row[1] + row[3] - 4)
    peaks = []
    for label, delta in [('first', 450), ('next', 900)]:
        frame = run.clock('advance', delta)
        image, capture = run.capture('waiting-' + label + '-peak')
        assert geometry(frame) == geometry(origin), 'waiting stole focus or moved its pane'
        count = changed(first, image, gap)
        assert count > 0, 'waiting loop did not repeat its swell'
        # Amber over the unchanged dark backdrop increases red more than blue.
        before, after = region(first, gap), region(image, gap)
        red = sum(after[i] - before[i] for i in range(0, len(after), 4))
        blue = sum(after[i + 2] - before[i + 2] for i in range(0, len(after), 4))
        assert red > blue and red > 0, 'waiting swell is not amber'
        row_count = changed(first, image, row_gap)
        assert row_count > 0, 'waiting row did not pulse with the pane'
        peaks.append({'frame': frame, 'capture': capture, 'halo_pixels': count,
                      'row_halo_pixels': row_count})
    run.ctl('ui', 'focus', seat)
    handled = run.clock('freeze')
    h0, ch0 = run.capture('handled-origin')
    single = run.clock('advance', 450)
    h1, ch1 = run.capture('handled-single-peak')
    assert changed(h0, h1, gap) > 0, 'focus did not play its one swell'
    assert changed(h0, h1, row_gap) > 0, 'focus did not play the row swell'
    done = run.clock('advance', 450)
    h2, ch2 = run.capture('handled-settled')
    later = run.clock('advance', 900)
    h3, ch3 = run.capture('handled-still-settled')
    assert changed(h0, h2, gap) == changed(h0, h3, gap) == 0, 'handled waiting loop restarted'
    assert changed(h0, h2, row_gap) == changed(h0, h3, row_gap) == 0, 'handled row loop restarted'
    assert geometry(handled) == geometry(single) == geometry(done) == geometry(later)
    run.ctl('seat', 'wait', seat, '--state', 'waiting', '--timeout', '20000')
    run.ctl('seat', 'type', seat, 'handled')
    run.ctl('seat', 'key', seat, 'enter')
    run.ctl('seat', 'wait', seat, '--text', '(?m)^R5-CLEAR-1$', '--timeout', '20000')
    run.ctl('seat', 'wait', seat, '--state', 'idle', '--timeout', '20000')
    # A separate unhandled episode ends when the harness leaves waiting,
    # without a focus action or a test-only state mutation.
    run.ctl('seat', 'type', seat, 'again')
    run.ctl('seat', 'key', seat, 'enter')
    run.ctl('seat', 'wait', seat, '--text', '(?m)^R5-WAIT-2$', '--timeout', '20000')
    run.ctl('seat', 'wait', seat, '--state', 'waiting', '--timeout', '20000')
    second_origin = run.clock('freeze')
    w0, cw0 = run.capture('second-episode-origin')
    second_peak = run.clock('advance', 450)
    w1, cw1 = run.capture('second-episode-peak')
    assert changed(w0, w1, gap) > 0, 'new unhandled waiting episode did not loop'
    assert changed(w0, w1, row_gap) > 0, 'new unhandled waiting row did not loop'
    run.ctl('seat', 'type', seat, 'done')
    run.ctl('seat', 'key', seat, 'enter')
    run.ctl('seat', 'wait', seat, '--text', '(?m)^R5-CLEAR-2$', '--timeout', '20000')
    run.ctl('seat', 'wait', seat, '--state', 'idle', '--timeout', '20000')
    cleared = run.clock('freeze')
    w2, cw2 = run.capture('second-episode-cleared')
    cleared_later = run.clock('advance', 900)
    w3, cw3 = run.capture('second-episode-stays-cleared')
    assert changed(w2, w3, gap) == 0, 'leaving waiting did not stop the loop'
    assert changed(w2, w3, row_gap) == 0, 'leaving waiting did not stop the row loop'
    assert geometry(second_origin) == geometry(second_peak) == geometry(cleared) == geometry(cleared_later)
    return {'promise': 'R5', 'origin': origin, 'loop_peaks': peaks,
            'handled': [handled, single, done, later], 'waiting_until_released': True,
            'cleared_episode': [second_origin, second_peak, cleared, cleared_later],
            'captures': [c0, ch0, ch1, ch2, ch3, cw0, cw1, cw2, cw3]}


def least_squares(columns, values):
    count = len(columns[0])
    matrix = [[sum(row[i] * row[j] for row in columns) for j in range(count)]
              + [sum(row[i] * v for row, v in zip(columns, values))] for i in range(count)]
    for col in range(count):
        pivot = max(range(col, count), key=lambda row: abs(matrix[row][col]))
        matrix[col], matrix[pivot] = matrix[pivot], matrix[col]
        scale = matrix[col][col]
        assert abs(scale) > 1e-10, 'wave sample basis is singular'
        matrix[col] = [value / scale for value in matrix[col]]
        for row in range(count):
            if row != col:
                factor = matrix[row][col]
                matrix[row] = [value - factor * other for value, other in zip(matrix[row], matrix[col])]
    return [row[-1] for row in matrix]


def rim_pixels(image, rect):
    width, height, pixels = image
    x, y, w, h = map(int, rect)
    points = ([(xx, yy) for xx in range(x + 16, x + w - 16) for yy in (y - 1, y + h)]
              + [(xx, yy) for yy in range(y + 16, y + h - 16) for xx in (x - 1, x + w)])
    assert all(0 <= xx < width and 0 <= yy < height for xx, yy in points)
    return [pixels[(yy * width + xx) * 4:(yy * width + xx) * 4 + 3] for xx, yy in points]


def rim_colour(run, seat, image, rect):
    # The public seat-colour rule, read from this build's kit. Averaging the
    # whole perimeter removes its two hue waves; no shader equation is copied.
    tokens = run.tokens
    config = tokens['seat_color']
    hashed = 2166136261
    for char in seat:
        hashed = ((hashed ^ ord(char)) * 16777619) & 0xffffffff
    hue = (hashed % config['hue_steps']) / config['hue_steps']
    light = config['bands_dark'][(hashed // config['hue_steps']) % len(config['bands_dark'])] / 100
    expected = colorsys.hls_to_rgb(hue, light, config['saturation'] / 100)
    expected_mean = sum(expected) / 3
    reference = [value - expected_mean for value in expected]
    background = bytes.fromhex(tokens['color']['dark']['bg'].lstrip('#'))
    vectors = []
    for pixel in rim_pixels(image, rect):
        signal = [value - bg for value, bg in zip(pixel, background)]
        mean = sum(signal) / 3
        chroma = [value - mean for value in signal]
        magnitude = math.sqrt(sum(value * value for value in chroma))
        if magnitude > 10:
            vectors.append([value / magnitude for value in chroma])
    assert len(vectors) > 100, 'rim has no measurable seat colour'
    actual = [sum(vector[k] for vector in vectors) / len(vectors) for k in range(3)]
    cosine = sum(a * b for a, b in zip(reference, actual)) / math.sqrt(sum(a * a for a in reference) * sum(b * b for b in actual))
    error = math.degrees(math.acos(min(1., max(-1., cosine))))
    assert error < 6, 'the rim colour is not the owning seat colour'
    return {'seat': seat, 'hue_degrees': hue * 360, 'perimeter_colour_error_degrees': error}


def activity_pulses(run):
    # OSC 7501 is fixture PTY output, not a UI state override. Each condition
    # holds at a read until the script releases it; no sleeps choose a phase.
    seat = run.shell('R1-activity', stable=False)
    command = r'''printf '\033[?25l'; for r1_state in idle working blocked error; do printf '\033]7501;state=%s:kind=question\007' "$r1_state"; printf '\nR1-STATE-%s\n' "$r1_state"; read r1_continue; done; exec cat'''
    run.line(seat, command)
    tokens = run.tokens
    background = bytes.fromhex(tokens['color']['dark']['bg'].lstrip('#'))
    measured = []
    for index, (wire, status, speed) in enumerate([('idle', 'idle', .35), ('working', 'active', 1.), ('blocked', 'waiting', .6), ('error', 'errored', 0.)]):
        run.ctl('seat', 'wait', seat, '--text', '(?m)^R1-STATE-' + wire + '$', '--timeout', '20000')
        if status in ('idle', 'waiting'):
            run.ctl('seat', 'wait', seat, '--state', status, '--timeout', '20000')
        actual = next(row for row in run.ctl('seat', 'ls') if row['record']['id'] == seat)
        assert actual['state']['activity']['kind'] == status, 'fixture activity was not reported'
        run.ctl('ui', 'focus', seat)  # handles the waiting episode, not its status
        start = run.clock('advance', 900)  # finish that focus's single swell
        rect = nodes(start, 'named:pane')[0]['rect']
        readings = []
        if wire == 'blocked':
            # Waiting adds its 1600ms breath to the activity pulse. Observe a
            # whole breath rather than pretending its maximum is another state's.
            deltas = [0] + [400] * 8
        else:
            # rim-glow §1: period 2600/(.5+speed). Fit a complete cycle
            # with a free phase: the seed is an implementation choice, not
            # a phase a script may guess from the seat id.
            period = 2600 / (.5 + speed)
            deltas = [0] + [round(period / 8)] * 8
        for sample, delta in enumerate(deltas):
            frame = run.clock('advance', delta)
            image, capture = run.capture('activity-' + wire + '-' + str(sample))
            assert geometry(frame) == geometry(start), 'activity pulse moved pane geometry'
            strength = sum(sum(value - bg for value, bg in zip(pixel, background)) for pixel in rim_pixels(image, rect))
            readings.append({'strength': strength, 'frame': frame, 'capture': capture})
        values = [entry['strength'] for entry in readings]
        if wire == 'blocked':
            assert max(values) > min(values) * 1.15, 'waiting rim does not breathe'
            pulse_fit = None
        else:
            basis = [[1., math.cos(entry['frame']['clock']['time_ms'] * 2 * math.pi / period),
                      math.sin(entry['frame']['clock']['time_ms'] * 2 * math.pi / period)] for entry in readings]
            offset, a, b = least_squares(basis, values)
            mean = sum(values) / len(values)
            variance = sum((value - mean) ** 2 for value in values)
            error = sum((value - sum(c * v for c, v in zip(row, (offset, a, b)))) ** 2 for row, value in zip(basis, values))
            fit = 1 - error / variance
            amplitude = math.hypot(a, b) / offset
            assert fit > .90 and amplitude > .10, 'activity pulse misses its documented period'
            pulse_fit = {'period_ms': period, 'r_squared': fit, 'relative_amplitude': amplitude, 'mean': offset}
        measured.append({'status': status, 'speed': speed, 'readings': readings, 'pulse_fit': pulse_fit})
        if index != 3:
            run.ctl('seat', 'type', seat, 'next')
            run.ctl('seat', 'key', seat, 'enter')
    # Idle and active have the same seat colour and focus; their amplitude,
    # unlike the pulse period, must get larger when work is reported.
    idle = measured[0]['pulse_fit']['mean']
    active = measured[1]['pulse_fit']['mean']
    assert 2.6 < active / idle < 3.6, 'active rim does not have its larger activity amplitude'
    return measured


def r1(run):
    # These assertions cover pane motion, gain and content protection. The
    # remaining clauses stay named gaps until their pixel predicates exist.
    seats = [run.shell('R1-first', tab=False), run.shell('R1-second', tab=False)]
    run.ctl('ui', 'focus', seats[0])
    focused = run.clock('advance', 900)
    assert len(focused['panes']) == 2, 'fixture needs two actual split panes'
    image_f, cf = run.capture('focused')
    run.ctl('ui', 'focus', seats[1])
    unfocused = run.clock('freeze')
    image_u, cu = run.capture('unfocused-same-time')
    assert focused['clock']['time_ms'] == unfocused['clock']['time_ms']
    target = next(p for p in focused['panes'] if p['seat'] == seats[0])
    center = target['rect'][0] + target['rect'][2] / 2
    rect = next(n['rect'] for n in nodes(focused, 'named:pane')
                if n['rect'][0] < center < n['rect'][0] + n['rect'][2])
    y = int(rect[1] - 1)
    xs = range(int(rect[0] + 40), int(rect[0] + rect[2] - 40))

    def profile(image):
        width, _, pixels = image
        return [sum(pixels[(y * width + x) * 4:(y * width + x) * 4 + 3]) for x in xs]

    u, f = profile(image_u), profile(image_f)
    ordered = sorted(u)
    lo, hi = ordered[len(ordered) // 10], ordered[len(ordered) * 6 // 10]
    pairs = [(a, b) for a, b in zip(u, f) if lo <= a <= hi]
    mean_u = sum(a for a, b in pairs) / len(pairs)
    mean_f = sum(b for a, b in pairs) / len(pairs)
    variance = sum((a - mean_u) ** 2 for a, b in pairs)
    assert variance > 0, 'rim edge has no spatial variation'
    gain = sum((a - mean_u) * (b - mean_f) for a, b in pairs) / variance
    intercept = mean_f - gain * mean_u
    fit = 1 - sum((b - gain * a - intercept) ** 2 for a, b in pairs) / sum((b - mean_f) ** 2 for a, b in pairs)
    assert abs(gain - 1.3) <= .04 and fit >= .98, 'focused rim does not apply its 1.3 gain'
    # The kit's pane radius and snapped 170/110px waves. Hue rotation is
    # about grey, so RGB sum keeps the intensity signal used for phase fitting.
    radius = run.style['radius']['row']
    perimeter = 2 * (rect[2] + rect[3]) - 8 * radius + 2 * math.pi * radius
    wavelengths = [perimeter / max(1, math.floor(perimeter / wave + .5)) for wave in (170., 110.)]
    basis = []
    for x in xs:
        row = [1.]
        for harmonic in (1, 2):
            for wavelength in wavelengths:
                angle = 2 * math.pi * x / wavelength * harmonic
                row.extend((math.sin(angle), math.cos(angle)))
        basis.append(row)
    run.clock('advance', 900)  # the peer focus has also finished its swell
    phases = [[], []]
    samples = []
    initial = final = None
    for index in range(7):
        frame = run.clock('freeze') if index == 0 else run.clock('advance', 200)
        image, capture = run.capture('wave-' + str(index))
        if initial is None:
            initial = image
        final = image
        coefficients = least_squares(basis, profile(image))
        for wave in range(2):
            phase = math.atan2(coefficients[2 + 2 * wave], coefficients[1 + 2 * wave])
            if phases[wave]:
                while phase - phases[wave][-1] > math.pi:
                    phase -= 2 * math.pi
                while phase - phases[wave][-1] < -math.pi:
                    phase += 2 * math.pi
            phases[wave].append(phase)
        samples.append({'frame': frame, 'capture': capture, 'coefficients': coefficients})
    deltas = [values[-1] - values[0] for values in phases]
    assert deltas[0] < -2 and deltas[1] > 1, 'the two rim waves do not counter-move'
    assert all(b < a for a, b in zip(phases[0], phases[0][1:]))
    assert all(b > a for a, b in zip(phases[1], phases[1][1:]))
    for outer in nodes(samples[0]['frame'], 'named:pane'):
        x, yy, width, height = outer['rect']
        band = (x + 32, yy - 1, x + width - 32, yy)
        assert changed(initial, final, band) > 0, 'a visible pane has no alive rim motion'
        # R/2 outside, the inner half and its padding-only
        # bloom. Compare time-separated captures so the static stroke cannot
        # masquerade as an alive band.
        half = run.tokens['glow']['rim'] / 2
        depth = min(run.tokens['glow']['depth'], run.style['spacing']['s4'])
        for offset in range(-math.floor(half), math.ceil(half)):
            assert changed(initial, final, (x + 32, yy + offset, x + width - 32, yy + offset + 1)) > 0, 'rim is not centred across its border'
        for offset in (-math.ceil(half) - 1, math.ceil(depth)):
            assert changed(initial, final, (x + 32, yy + offset, x + width - 32, yy + offset + 1)) == 0, 'rim escaped its outer half or 8px padding'
    for strip in nodes(samples[0]['frame'], 'named:pane/strip'):
        x, yy, width, height = strip['rect']
        assert changed(initial, final, (x + 32, yy - 1, x + width - 32, yy)) > 0, 'a title strip has no alive top edge'
    for pane in samples[0]['frame']['panes']:
        x, yy, width, height = pane['rect']
        box = (x, yy, x + width, yy + height)
        assert region(initial, box) == region(final, box), 'alive rim covered terminal content'
    assert all(geometry(sample['frame']) == geometry(samples[0]['frame']) for sample in samples), 'alive rim animation moved pane geometry'
    colours = []
    for pane in samples[0]['frame']['panes']:
        center = pane['rect'][0] + pane['rect'][2] / 2
        outer = next(n['rect'] for n in nodes(samples[0]['frame'], 'named:pane') if n['rect'][0] < center < n['rect'][0] + n['rect'][2])
        colours.append(rim_colour(run, pane['seat'], initial, outer))
    pulses = activity_pulses(run)
    return {'promise': 'R1', 'focus_gain': gain, 'focus_fit_r_squared': fit,
            'wave_phases': phases, 'wave_deltas': deltas, 'samples': samples,
            'captures': [cf, cu], 'content_identical': True,
            'seat_colours': colours, 'activity_pulses': pulses,
            'coverage_gaps': []}


def main():
    promise = sys.argv[1]
    driver = {'R1': r1, 'R3': r3, 'R5': r5}[promise]
    run = Run(promise)
    verdict = None
    try:
        run.start()
        verdict = driver(run)
    finally:
        run.finish(verdict)
    print(json.dumps({'promise': promise, 'verdict': 'pass',
                      'evidence': str(run.out / 'verdict.json'),
                      'coverage_gaps': verdict.get('coverage_gaps', [])}))


if __name__ == '__main__':
    main()
