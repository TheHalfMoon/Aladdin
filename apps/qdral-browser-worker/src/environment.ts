// SG-000108 worker environment hygiene.
//
// The host must start the worker with an environment built from an
// allowlist, never inherited: Node itself acts on variables such as
// NODE_OPTIONS (for example `--require`) before any worker code runs, so this
// in-process check cannot stop them (verified natively). It is a second line
// for variables Node does not consume at startup (Playwright debugging and
// download switches, proxies, key logging) and makes a misconfigured launch
// fail loudly. Names are compared case-insensitively, as on Windows.

const REFUSED_PREFIXES = ["NODE_", "PLAYWRIGHT_", "PW_", "ELECTRON_"];
const REFUSED_NAMES = ["DEBUG", "DEBUG_COLORS", "DEBUG_FD", "UV_THREADPOOL_SIZE", "SSLKEYLOGFILE", "HTTP_PROXY", "HTTPS_PROXY", "ALL_PROXY", "NO_PROXY"];

/// Names of refused variables present in `env` (empty when clean).
export function refusedWorkerVariables(env: Record<string, string | undefined>): string[] {
  const refused: string[] = [];
  for (const [name, value] of Object.entries(env)) {
    if (value === undefined) continue;
    const upper = name.toUpperCase();
    if (REFUSED_PREFIXES.some((prefix) => upper.startsWith(prefix)) || REFUSED_NAMES.includes(upper)) {
      refused.push(name);
    }
  }
  return refused.sort();
}
