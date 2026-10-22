// Acceptance witnesses only. These diagnostics never enter native layout.
export function captureExercises(c) {
  if (c.component !== 'components' || !['flow', 'text-align'].includes(c.state)) return;
  const tree = c.fixture.component.tree;
  const baseline = n => n.font_diagnostics?.baseline != null
    ? n.rect.y + n.font_diagnostics.baseline
    : (n.children || []).map(baseline).find(v => v != null);
  if (c.state === 'flow') {
    const rows = tree.children.map(n => {
      const lines = [];
      for (const child of n.children) {
        const b = baseline(child);
        if (!Number.isFinite(b)) throw Error('Flow reference is missing a baseline witness');
        if (!lines.some(y => Math.abs(y - b) <= .5)) lines.push(b);
      }
      return lines.length;
    });
    c.fixture.exercise = {rows, expected: [2, 2, 1]};
    if (JSON.stringify(rows) !== '[2,2,1]') throw Error('Flow reference must exercise two wrapped rows, two chip rows and one space-between row: ' + rows);
  } else {
    const text = tree.children.at(-1), f = text.font_diagnostics;
    const wrapped = text.rect.h > 1.5 * (f.ascent + f.descent);
    c.fixture.exercise = {wrapped_caption: wrapped};
    if (!wrapped) throw Error('Text alignment reference must exercise a wrapped centred caption');
  }
}
