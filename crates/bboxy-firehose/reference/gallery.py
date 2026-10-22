#!/usr/bin/env python3
"""Build local, true-size web | native | diff evidence. Never rescales an image."""
from pathlib import Path
import hashlib
import html
import json
import numpy as np
from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parent / 'gallery'
before = json.loads((ROOT / 'before/manifest.json').read_text())
native = json.loads((ROOT / 'native/manifest.json').read_text())
before_cases = {case['variant']: case for case in before['cases']}
for sub in ['diff', 'read']:
    (ROOT / sub).mkdir(exist_ok=True)


def canvas(image, size, background):
    result = Image.new('RGBA', size, background)
    result.paste(image, (0, 0))
    return np.asarray(result).astype(np.int16)


cases = []
font = ImageFont.truetype('/System/Library/Fonts/Menlo.ttc', 13)
for case in native['cases']:
    variant, file = case['variant'], case['file']
    previous = before_cases[variant]
    web_image = Image.open(ROOT / 'web' / file).convert('RGBA')
    native_image = Image.open(ROOT / 'native' / file).convert('RGBA')
    old_image = Image.open(ROOT / 'before' / file).convert('RGBA')
    size = (max(web_image.width, native_image.width, old_image.width),
            max(web_image.height, native_image.height, old_image.height))
    background = (242, 244, 248, 255) if '-light-' in variant else (15, 20, 29, 255)
    web = canvas(web_image, size, background)
    current = canvas(native_image, size, background)
    old = canvas(old_image, size, background)
    difference = np.max(np.abs(web[:, :, :3] - current[:, :, :3]), axis=2)
    old_difference = np.max(np.abs(web[:, :, :3] - old[:, :, :3]), axis=2)
    changed = difference > 12
    old_changed = old_difference > 12
    viewport_changed = changed[:web_image.height, :web_image.width]
    heat = np.zeros_like(web, dtype=np.uint8)
    heat[:, :, 0] = np.minimum(difference * 4, 255)
    heat[:, :, 1] = np.minimum(difference, 255)
    heat[:, :, 2] = np.minimum(difference * 2, 255)
    heat[:, :, 3] = 255
    diff_image = Image.fromarray(heat, 'RGBA')
    diff_image.save(ROOT / 'diff' / file)
    # Original pixel sizes, additional empty margins only; no resize, no zoom.
    strip = Image.new('RGB', (size[0] * 3 + 32, size[1] + 34), '#20252f')
    draw = ImageDraw.Draw(strip)
    for index, (title, image) in enumerate([('xterm.js', web_image), ('native cells', native_image), ('difference ×4', diff_image)]):
        x = index * (size[0] + 16)
        draw.text((x + 6, 8), title, font=font, fill='white')
        strip.paste(image.convert('RGB'), (x, 34))
    strip.save(ROOT / 'read' / file)
    cases.append({**case,
        'webSize': list(web_image.size), 'nativeSize': list(native_image.size),
        'beforeSize': list(old_image.size), 'comparisonCanvas': list(size),
        'beforeChangedPercent': float(old_changed.mean() * 100),
        'changedPercent': float(changed.mean() * 100),
        'viewportChangedPercent': float(viewport_changed.mean() * 100),
        'changedPixels': int(changed.sum()), 'tolerance': 12,
        'geometryMatch': web_image.size == native_image.size,
        'beforeLegacyError': previous.get('legacyError'),
        'beforeDiagnosticReplacements': previous.get('diagnosticReplacements', 0),
        'webSha256': hashlib.sha256((ROOT / 'web' / file).read_bytes()).hexdigest(),
        'nativeSha256': hashlib.sha256((ROOT / 'native' / file).read_bytes()).hexdigest(),
    })

groups = {}
for case in cases:
    groups.setdefault(case['name'], []).append(case)
summary = [{
    'scene': name,
    'beforeMeanPercent': sum(c['beforeChangedPercent'] for c in group) / len(group),
    'afterMeanPercent': sum(c['changedPercent'] for c in group) / len(group),
    'afterWorstPercent': max(c['changedPercent'] for c in group),
    'afterViewportMeanPercent': sum(c['viewportChangedPercent'] for c in group) / len(group),
    'geometryMatches': sum(c['geometryMatch'] for c in group),
    'beforeRenderFailures': sum(bool(c['beforeLegacyError']) for c in group),
} for name, group in groups.items()]
report = {
    'metric': 'max absolute RGB channel difference >12/255; same padded union canvas before/after, no resampling',
    'baseline': 'actual legacy renderer where supported; visibly labelled ASCII-replacement diagnostic where original rendering errored',
    'cases': cases, 'perScene': summary,
}
(ROOT / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
rows = []
for case in sorted(cases, key=lambda case: case['changedPercent'], reverse=True):
    variant, file = html.escape(case['variant']), html.escape(case['file'])
    warning = ''
    if case['beforeLegacyError']:
        warning = f" Legacy failed: {html.escape(case['beforeLegacyError'])}; before PNG uses {case['beforeDiagnosticReplacements']} diagnostic replacements."
    rows.append(f'''<section><h2>{variant}</h2><p>Before {case['beforeChangedPercent']:.3f}% → after {case['changedPercent']:.3f}% on the shared comparison canvas; native viewport {case['viewportChangedPercent']:.3f}%; geometry {'matches' if case['geometryMatch'] else 'DIFFERS'}.{warning} <a href="before/{file}">Before image</a></p><div class="comparison"><figure><figcaption>xterm.js</figcaption><img src="web/{file}"></figure><figure><figcaption>Native cells</figcaption><img src="native/{file}"></figure><figure><figcaption>Difference ×4</figcaption><img src="diff/{file}"></figure></div></section>''')
(ROOT / 'index.html').write_text('''<!doctype html><meta charset="utf-8"><title>Terminal fidelity</title><style>body{background:#0f141d;color:#e8edf7;font:13px Menlo,monospace;margin:24px}h1{font-size:22px}h2{font-size:16px}p{line-height:1.6}a{color:#40b4ff}.comparison{display:flex;gap:16px;overflow:auto}figure{margin:0;flex:none}figcaption{margin:8px 0}img{display:block;max-width:none}section{margin:32px 0;border-top:1px solid #364254}</style><h1>Terminal fidelity</h1><p>Seed 42 · Menlo 13 · 80×24 · dark/light · 1x/2x. Images retain original pixels. Metric: channel difference &gt;12/255 on the same padded comparison canvas. Actual iTerm2/Ghostty comparison remains unproven.</p>''' + '\n'.join(rows))
for scene in summary:
    print(f"{scene['scene']}: before={scene['beforeMeanPercent']:.3f}% after={scene['afterMeanPercent']:.3f}% worst={scene['afterWorstPercent']:.3f}% geometry={scene['geometryMatches']}/4 baseline_failures={scene['beforeRenderFailures']}/4")
