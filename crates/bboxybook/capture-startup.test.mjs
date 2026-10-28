// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {once} from 'node:events';
import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import {waitForDevToolsPort} from './capture-startup.mjs';

test('capture accepts a Chrome process that publishes its port after ten seconds', {timeout: 120_000}, async () => {
  const profile = await fs.mkdtemp(path.join(os.tmpdir(), 'bboxybook-startup-test-'));
  const binary = path.join(profile, 'slow-chrome.cjs');
  await fs.writeFile(binary, `setTimeout(() => require('node:fs').writeFileSync(process.argv[2], '12345\\n'), 10000);\n`);
  const child = spawn(process.execPath, [binary, path.join(profile, 'DevToolsActivePort')], {stdio: 'ignore'});
  const exited = once(child, 'exit');
  try {
    assert.equal(await waitForDevToolsPort(profile, () => ''), 12345);
    assert.deepEqual(await exited, [0, null]);
  } finally {
    if (child.exitCode === null && child.signalCode === null) child.kill('SIGTERM');
    await exited;
    await fs.rm(profile, {recursive: true, force: true});
  }
});

test('startup guard preserves Chrome stderr in the failure', async () => {
  await assert.rejects(waitForDevToolsPort('/unused-profile', () => 'cold-cache failure', 0),
    /Chrome did not start.*startup guard: cold-cache failure/);
});
