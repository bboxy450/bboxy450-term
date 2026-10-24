"""Verify that default A/B changes stay inside the approved Creativity rails."""
from pathlib import Path
from PIL import Image, ImageChops, ImageDraw
import hashlib
import json
import math
import sys

root, captured, output = map(Path, sys.argv[1:4])
report = json.loads((root / 'byte-identity.json').read_text())
cases = json.loads(captured.read_text())
keys = set(report['different'])
rows = []
for case in cases:
    key = f"{case['component']}/{case['slug']}-{case['density']}-{case['theme']}.png"
    if key not in keys:
        continue
    before = Image.open(root / 'baseline/native' / key).convert('RGBA')
    after = Image.open(root / 'candidate/native' / key).convert('RGBA')
    channels = ImageChops.difference(before, after).split()
    changed = channels[0]
    for channel in channels[1:]:
        changed = ImageChops.lighter(changed, channel)
    allowed = Image.new('L', before.size)
    draw = ImageDraw.Draw(allowed)
    boxes = []
    for frame in case['fixture']['frames']:
        for index, entry in enumerate(frame['view']['entries']):
            if entry.get('kind') != 'message' or entry.get('data', {}).get('role') != 'creativity':
                continue
            part = next(p for p in frame['parts'] if p['key'] == f'e{index}/rail')
            rect = part['rect']
            box = [math.floor(rect['x']), math.floor(rect['y']), math.ceil(rect['x'] + rect['w']), math.ceil(rect['y'] + rect['h'])]
            boxes.append(box)
            draw.rectangle([box[0], box[1], box[2] - 1, box[3] - 1], fill=255)
    outside = ImageChops.subtract(changed, allowed)
    rows.append({'key': key, 'changed_bbox': changed.getbbox(), 'captured_creativity_rail_boxes': boxes,
                 'changed_pixels': sum(changed.histogram()[1:]), 'outside_rails_pixels': sum(outside.histogram()[1:]),
                 'outside_rails_bbox': outside.getbbox()})
result = {'before_source': report['before_source'], 'after_source': report['after_source'],
          'input_cases_sha256': hashlib.sha256(captured.read_bytes()).hexdigest(), 'cases': report['cases'],
          'byte_identical': report['identical'], 'changed': len(rows),
          'method': 'Every RGBA difference must be inside the preserved input capture Creativity rail rectangles; no pixel allowance elsewhere.',
          'approval_message': '93dbb857-8ba6-44b0-92a6-a388f6906e3b', 'source_rail_commit': '1fae4199',
          'all_changed_pixels_within_rails': len(rows) == len(keys) and all(row['captured_creativity_rail_boxes'] and row['changed_pixels'] > 0 and row['outside_rails_pixels'] == 0 for row in rows), 'rows': rows}
output.write_text(json.dumps(result, indent=2) + '\n')
print(json.dumps({key: value for key, value in result.items() if key != 'rows'}))
raise SystemExit(0 if result['all_changed_pixels_within_rails'] else 1)
