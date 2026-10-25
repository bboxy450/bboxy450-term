import fs from 'node:fs/promises';
import path from 'node:path';

const pause = ms => new Promise(resolve => setTimeout(resolve, ms));

export async function waitForDevToolsPort(profile, stderr, timeoutMs = 60_000) {
  const deadline = performance.now() + timeoutMs;
  while (performance.now() < deadline) {
    try {
      const port = Number((await fs.readFile(path.join(profile, 'DevToolsActivePort'), 'utf8')).split('\n')[0]);
      if (Number.isInteger(port) && port > 0 && port <= 65535) return port;
    } catch {}
    await pause(Math.min(50, Math.max(0, deadline - performance.now())));
  }
  throw Error(`Chrome did not start within ${timeoutMs} ms startup guard: ${stderr()}`);
}
