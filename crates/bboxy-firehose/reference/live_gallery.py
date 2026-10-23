#!/usr/bin/env python3
"""Compare actual live pane crops with pinned xterm and headless cells."""
import argparse
import hashlib
import html
import json
from pathlib import Path
import shutil

import numpy as np
from PIL import Image, ImageChops, ImageDraw


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--live', type=Path, required=True)
    parser.add_argument('--headless', type=Path, required=True)
    parser.add_argument('--web', type=Path, default=Path(__file__).parent / 'gallery/web')
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    args.out.mkdir()  # Previous evidence must not be overwritten.
    for sub in ['web', 'native', 'diff', 'read']:
        (args.out / sub).mkdir()
    manifest = json.loads((args.live / 'manifest.json').read_text())
    report, sections = [], []
    for row in manifest['cases']:
        name = f"{row['scene']}-{row['theme']}-1x"
        capture = args.live / name / 'capture.png'
        if not row['match_expected'] or row['remaining_pids'] or sha(capture) != row['png_sha256']:
            raise ValueError(f'{name}: unverified capture or incomplete cleanup')
        grid = row['grid']
        if grid['scale'] != 1 or len(grid['panes']) != 1:
            raise ValueError(f'{name}: expected one actual 1x pane')
        pane = grid['panes'][0]
        x, y, w, h = pane['crop']
        full = Image.open(capture).convert('RGB')
        if x + w > full.width or y + h > full.height:
            raise ValueError(f'{name}: crop extends beyond capture')
        live = full.crop((x, y, x + w, y + h))
        web_path = args.web / (name + '.png')
        web = Image.open(web_path).convert('RGB')
        head = Image.open(args.headless / (name + '.png')).convert('RGB')
        if live.size != web.size or live.size != head.size:
            raise ValueError(f'{name}: comparison would require resampling')
        shutil.copyfile(web_path, args.out / 'web' / (name + '.png'))
        live.save(args.out / 'native' / (name + '.png'))
        difference = ImageChops.difference(web, live)
        raw = np.any(np.asarray(head) != np.asarray(live), axis=2)
        outside = raw.copy()
        cy, cx = pane['cursor']
        cw, ch = w // pane['cols'], h // pane['rows']
        outside[cy * ch:(cy + 1) * ch, cx * cw:(cx + 1) * cw] = False
        changed = np.max(np.asarray(difference), axis=2) > 12
        heat = difference.point(lambda p: min(255, p * 4))
        heat.save(args.out / 'diff' / (name + '.png'))
        strip = Image.new('RGB', (w * 3 + 32, h + 30), '#202630')
        draw = ImageDraw.Draw(strip)
        for i, (label, im) in enumerate([
            ('xterm.js primary', web), ('live app 1x', live), ('difference x4', heat),
        ]):
            strip.paste(im, (i * (w + 16), 30))
            draw.text((i * (w + 16) + 4, 8), label, fill='white')
        path = args.out / 'read' / (name + '.png')
        strip.save(path)
        item = dict(variant=name, screen_hash=pane['fnv1a64'], capture_sha256=sha(capture),
                    changed_gt12_percent=float(changed.mean() * 100),
                    changed_vs_headless_pixels=int(raw.sum()),
                    changed_vs_headless_outside_cursor=int(outside.sum()),
                    strip_sha256=sha(path), image_read_by_agent=False)
        report.append(item)
        sections.append(f'<section><h2>{html.escape(name)}</h2><p>'
                        f'xterm pixel difference {item["changed_gt12_percent"]:.3f}%; '
                        f'live/headless outside cursor: {int(outside.sum())} pixels.</p>'
                        f'<img src="read/{html.escape(name)}.png"></section>')
        print(json.dumps(item))
    (args.out / 'report.json').write_text(json.dumps(dict(
        method='Original pixels; actual model crop; max RGB difference >12/255. '
               'Focused-cursor exclusion applies only to separately named headless metric.',
        source=manifest['source'], cases=report), indent=2) + '\n')
    (args.out / 'index.html').write_text(
        '<!doctype html><meta charset="utf-8"><title>Live terminal audit</title>'
        '<style>body{background:#0f141d;color:#e8edf7;font:13px Menlo,monospace;'
        'margin:24px}section{margin:24px 0;overflow:auto}img{max-width:none}</style>'
        '<h1>Live terminal audit</h1><p>Seed42, Menlo13, actual 1x live panes. '
        'Headless2x is a separate gate; live2x and iTerm2/Ghostty remain unproven.</p>'
        + '\n'.join(sections))


if __name__ == '__main__':
    main()
