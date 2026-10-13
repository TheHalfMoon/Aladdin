// A fresh installed-Edge browser for one attempt: headless, a throwaway
// profile, loopback DevTools on an OS-chosen port, fixture.test only.
import { spawn } from 'node:child_process';
import { appendFileSync, existsSync, mkdtempSync, readFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const EDGE = [
  'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe',
  'C:/Program Files/Microsoft/Edge/Application/msedge.exe',
].find((path) => existsSync(path));

// run.mjs ends every browser recorded here (its whole process tree) and
// removes its profile once the suite is over, then verifies nothing remains.
export async function freshEdge(signal: AbortSignal): Promise<string> {
  if (!EDGE) throw new Error('Edge is not installed');
  const spki = process.env.FIXTURE_SPKI;
  if (!spki) throw new Error('FIXTURE_SPKI is not set (run through run.mjs)');
  const profile = mkdtempSync(join(tmpdir(), 'qdral-a8-edge-'));
  const child = spawn(
    EDGE,
    [
      '--headless=new',
      `--user-data-dir=${profile}`,
      '--remote-debugging-address=127.0.0.1',
      '--remote-debugging-port=0',
      '--no-first-run',
      '--no-default-browser-check',
      '--disable-extensions',
      '--disable-background-networking',
      '--disable-sync',
      '--disable-component-update',
      // Fixture origin only: every other name fails to resolve.
      '--host-resolver-rules=MAP fixture.test 127.0.0.1, MAP * ~NOTFOUND',
      `--ignore-certificate-errors-spki-list=${spki}`,
      'about:blank',
    ],
    { stdio: 'ignore' },
  );

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
