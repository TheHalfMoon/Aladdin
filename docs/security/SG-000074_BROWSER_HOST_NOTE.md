# SG-000074 Live Isolated Browser Host

Status: SG-000074 DESIGN RECORD (QDRAL-P18)
Date: 2026-10-04
Authority delta: one Deskal-supervised low-authority host family with no
reachable capability. No MCP tool, no navigation, no observation, no
actuation, no transfer, no capture, and no remote authority.

## 1. Shape

`crates/qdral-browser-host` owns engine discovery (`discovery.rs`),
dedicated ephemeral profiles (`profile.rs`), the frozen engine command
line (`argv.rs`), the private framed channel vocabulary (`proto.rs`),
supervised launch and deterministic cleanup (`supervise.rs`), single-host
supervision (`host.rs`), and the `qdral-browser-host` binary (`main.rs`).

Deskal supervision spawns the host binary with piped stdio and assigns it
to a kill-on-close job. The host binary re-verifies engine and profile,
spawns the engine suspended into a nested kill-on-close job with a bounded
process count, and serves only hello, ping, and shutdown frames. Pipe
end-of-input stops a host whose parent died, independent of job timing.

## 2. Isolation controls

- Engine sources: installed Edge or Chromium-family executables found
  through App Paths registration and well-known install locations, plus
  isolated test roots. Bundled, downloaded, and caller-supplied engines do
  not exist.
- Engine verification: allowlisted executable name, version probe with a
  ten-second bound, and Authenticode signature verification on Windows.
  Anything else is typed unavailable.
- Command line: exactly the frozen flags with the Deskal-owned profile
  directory and a blank initial page. Sandbox-weakening flags, debugging
  listeners, extension loading, and web-security relaxations fail closed.
- Profile: fresh ephemeral directory per launch with an engine-bound
  marker. Foreign and personal directories are never adopted. Profiles are
  deleted on clean shutdown.
- Environment: SystemRoot, SystemDrive, minimal system PATH, and temp
  only. USERPROFILE, APPDATA, and LOCALAPPDATA are deliberately absent so
  the engine can never resolve the personal profile, and no Deskal secret
  variable is inherited.
- Channel: anonymous pipes only. No TCP listener, no HTTP surface, no
  debugging port, no MCP endpoint. Unknown frames are rejected without
  action.
- Bounds: one live host per supervisor; at most 32 processes per browser
  job; 64 KiB frames; ten-second launch probe; five-second handshake;
  ten-second shutdown confirmation.

## 3. Platform boundaries

Windows carries the full guarantee: suspended launch, job assignment
before resume, kill-on-close chains, ActiveProcessLimit, and signature
verification. Off Windows the supervisor launches with handle supervision
and drop-kill, discovery covers well-known locations, and publisher
verification reports unverified. The hard parent-death guarantee off
Windows is a documented gap: supervised shutdown is deterministic, but a
violently killed supervisor may orphan the engine there. Windows
qualification is the normative path for this grain.

## 4. Unreachability proof

No browser capability is exposed: the MCP surface is unchanged, the tool
contract is unchanged, and the host vocabulary (hello, ping, shutdown)
cannot navigate, observe, actuate, transfer, or capture. The donor marker
scan covers the new crate, and the new integration tests prove launch
supervision and cleanup on machines with a working engine and typed
unavailability elsewhere. Missing and unsupported engines return typed
unavailable; there is no personal-browser fallback.

## 5. Packaging note

The host binary is built and tested with the workspace but is not yet in
the release payload: nothing launches it in production until SG-000075
wires navigation through it. Payload inclusion arrives with the first
grain that supervises a host outside tests.
