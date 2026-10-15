// SG-000108 slice 4: take the engine's DOM snapshot within a deadline.
//
// On a huge page the engine can still be serialising when the deadline
// passes, and detaching the CDP session then waits behind it (measured: 9 s
// after a 3 s deadline). So on a timeout the detach is never awaited, and
// otherwise it is bounded: the caller gets its answer on time either way.

/// The two calls discovery needs from a CDP session.
export interface SnapshotSession {
  capture(): Promise<unknown>;
  detach(): Promise<void>;
}

/// Upper bound on waiting for a detach after a capture that finished.
export const DETACH_WAIT_MS = 200;

export async function captureWithin(session: SnapshotSession, timeoutMs: number): Promise<unknown> {
  let timer: NodeJS.Timeout | undefined;
  let timedOut = false;
  const capture = session.capture();
  capture.catch(() => {});
  try {
    const deadline = new Promise<never>((_, reject) => {
      timer = setTimeout(() => {
        timedOut = true;
        reject(new Error("the DOM snapshot timed out"));
      }, timeoutMs);
    });
    return await Promise.race([capture, deadline]);
  } finally {
    clearTimeout(timer);
    const detached = session.detach().catch(() => {});
    if (!timedOut) {
      let wait: NodeJS.Timeout | undefined;
      await Promise.race([detached, new Promise<void>((resolve) => (wait = setTimeout(resolve, DETACH_WAIT_MS)))]);
      clearTimeout(wait);
    }
  }
}
