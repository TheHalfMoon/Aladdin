import { serveStdio } from "@modelcontextprotocol/server/stdio";
import { KernelClient } from "../kernel.js";
import {
  buildCotraServer,
  closeAllServerKernels,
  defaultTransportContext
} from "../server.js";

const DEFAULT_WORKSPACE = process.env.COTRA_DEFAULT_WORKSPACE ?? "default";

/**
 * Start the supported local stdio transport using the authoritative builder.
 *
 * This is the only transport that creates tools in v0.1, and it does so
 * exclusively through `buildCotraServer`. It adds no independent tool
 * authority and no transport-controlled tool fields.
 */
export function startStdioTransport() {
  const tracked = new Set<KernelClient>();

  function createServer() {
    const kernel = new KernelClient();
    tracked.add(kernel);
    const server = buildCotraServer(kernel, DEFAULT_WORKSPACE, defaultTransportContext());
    const previousOnClose = server.server.onclose;
    server.server.onclose = () => {
      tracked.delete(kernel);
      if (previousOnClose) {
        previousOnClose();
      } else {
        kernel.close();
      }
    };
    return server;
  }

  const handle = serveStdio(createServer);
  process.stderr.write("[cotra-mcp] serving Cotra SG-000017 tools over stdio\n");

  function shutdown(): void {
    for (const client of tracked) {
      try {
        client.close();
      } catch {
        // Fail closed on shutdown.
      }
    }
    tracked.clear();
    closeAllServerKernels();
    void handle.close().finally(() => process.exit(0));
  }

  process.on("SIGINT", shutdown);
  process.on("SIGTERM", shutdown);

  return { handle, shutdown };
}
