import { startStdioTransport } from "../transports/stdio.js";

/**
 * Supported local stdio entrypoint.
 *
 * Preserves Orcel v0.1 stdio behavior through the authoritative builder.
 * Future `orcel mcp stdio` lifecycle wiring (SG-000049) reuses this entrypoint
 * without changing tool authority.
 */
startStdioTransport();
