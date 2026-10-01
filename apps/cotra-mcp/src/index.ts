import { startStdioTransport } from "./transports/stdio.js";

/**
 * Cotra MCP stdio entrypoint (v0.1 behavior preserved).
 *
 * The authoritative tool catalog lives in `./server.js` and result
 * projection lives in `./result.js`. This module performs no tool
 * registration itself; the stdio transport reuses the authoritative builder.
 */
startStdioTransport();
