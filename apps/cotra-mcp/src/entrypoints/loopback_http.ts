import { startLoopbackTransport } from "../transports/loopback_http.js";

/**
 * Loopback HTTP entrypoint for `cotra mcp serve` and direct launches.
 *
 * Reads `COTRA_LOOPBACK_TOKEN` and `COTRA_LOOPBACK_PORT` from the protected
 * environment, binds `127.0.0.1` only, and serves the authoritative catalog.
 * Fails closed with a non-zero exit when the credential or port is invalid.
 */
function portFromEnv(value: string | undefined): number | undefined {
  if (value === undefined || value === "") {
    return undefined;
  }
  const parsed = Number(value);
  if (!Number.isInteger(parsed)) {
    return NaN;
  }
  return parsed;
}

const port = portFromEnv(process.env.COTRA_LOOPBACK_PORT);
if (port !== undefined && Number.isNaN(port)) {
  process.stderr.write("cotra mcp serve: COTRA_LOOPBACK_PORT must be an integer\n");
  process.exit(2);
}

try {
  await startLoopbackTransport({ port, token: process.env.COTRA_LOOPBACK_TOKEN });
} catch (error) {
  const message = error instanceof Error ? error.message : String(error);
  process.stderr.write(`cotra mcp serve: ${message}\n`);
  process.exit(1);
}
