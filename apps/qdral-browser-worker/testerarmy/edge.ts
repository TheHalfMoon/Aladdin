// A fresh installed-Edge browser for one attempt, under the production
// launch: run.mjs builds the exact argv and engine environment the host
// would hand the worker for the fixture.test destination, and has the
// worker's own contract check (launchRefusal) accept them. The one
// difference is attachment: TesterArmy needs a DevTools endpoint, so the
// pipe flag is replaced by DevTools on loopback at an OS-chosen port.
import { spawn } from 'node:child_process';
import { appendFileSync, existsSync, mkdirSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const EDGE = [
  'C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe',
  'C:\\Program Files\\Microsoft\\Edge\\Application\\msedge.exe',
].find((path) => existsSync(path));

// run.mjs ends every browser recorded here (its whole process tree) and
// removes its profile once the suite is over, then verifies nothing remains.
export async function freshEdge(signal: AbortSignal): Promise<string> {
  if (!EDGE) throw new Error('Edge is not installed');
  const template = JSON.parse(process.env.A8_ARGV ?? 'null') as string[] | null;
  const env = JSON.parse(process.env.A8_ENGINE_ENV ?? 'null') as Record<string, string> | null;
  if (!template || !env) throw new Error('A8_ARGV or A8_ENGINE_ENV is not set (run through run.mjs)');
  const profile = mkdtempSync(join(tmpdir(), 'qdral-a8-edge-'));
  // As the host seeds a profile: no network prediction (preconnect).
  mkdirSync(join(profile, 'Default'));
  writeFileSync(join(profile, 'Default', 'Preferences'), '{"net":{"network_prediction_options":2}}');
  const argv = template.flatMap((arg) => {
    if (arg === '--user-data-dir={PROFILE}') return [`--user-data-dir=${profile}`];
    if (arg === '--remote-debugging-pipe') return ['--remote-debugging-address=127.0.0.1', '--remote-debugging-port=0'];
    return [arg];
  });
  const child = spawn(EDGE, argv, { stdio: 'ignore', env });

  const portFile = join(profile, 'DevToolsActivePort');
  const deadline = Date.now() + 20_000;
  while (!existsSync(portFile) || readFileSync(portFile, 'utf8').split('\n').length < 2) {
    if (signal.aborted || Date.now() > deadline) throw new Error('Edge did not open DevTools');
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  const [port, path] = readFileSync(portFile, 'utf8').split('\n');
  const endpoint = `ws://127.0.0.1:${port.trim()}${path.trim()}`;
  if (process.env.A8_PIDS && child.pid !== undefined) {
    appendFileSync(process.env.A8_PIDS, `${child.pid}\t${profile}\t${endpoint}\n`);
  }
  return endpoint;
}
