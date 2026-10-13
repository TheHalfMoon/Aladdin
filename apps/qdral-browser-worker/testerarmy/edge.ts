// A fresh installed-Edge browser for one attempt: headless, a throwaway
// profile, loopback DevTools on an OS-chosen port, and the production
// network confinement (run.mjs hands over the worker's own flag values):
// fixture.test is the only admitted destination, every other name fails to
// resolve, and everything not admitted (IP literals included) goes to the
// no-route proxy.
import { spawn } from 'node:child_process';
import { appendFileSync, existsSync, mkdirSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
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
  const confinement = JSON.parse(process.env.A8_CONFINEMENT ?? 'null') as string[] | null;
  if (!spki || !confinement) throw new Error('FIXTURE_SPKI or A8_CONFINEMENT is not set (run through run.mjs)');
  const profile = mkdtempSync(join(tmpdir(), 'qdral-a8-edge-'));
  // As the host seeds a profile: no network prediction (preconnect).
  mkdirSync(join(profile, 'Default'));
  writeFileSync(join(profile, 'Default', 'Preferences'), '{"net":{"network_prediction_options":2}}');
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
      // The host's grammar for one admitted destination (confinement.rs).
      '--host-resolver-rules=MAP fixture.test 127.0.0.1:443, MAP * ~NOTFOUND',
      '--proxy-bypass-list=fixture.test;<-loopback>',
      ...confinement,
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
