import { startStdioTransport } from "../transports/stdio.js";

/**
 * Supported local stdio entrypoint.
 *
 * Preserves Quntal v0.1 stdio behavior through the authoritative builder.
 * Future `quntal mcp stdio` lifecycle wiring (SG-000049) reuses this entrypoint
 * without changing tool authority.
 */
startStdioTransport();
