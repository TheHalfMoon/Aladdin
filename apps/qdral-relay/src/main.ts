/**
 * SG-000057 self-host relay entrypoint.
 *
 *   node dist/main.js --config /path/to/relay.json
 *
 * Starts the open-source relay: authorization server, public `/mcp` edge,
 * and device channel, with durable state in the configured directory. Logs
 * are route and status classes only.
 */
import { loadRelayConfig } from "./config.js";
import { FileRelayStore } from "./file_store.js";
import { createRelayServer } from "./server.js";

function main(argv: readonly string[]): void {
  const index = argv.indexOf("--config");
  const path = index >= 0 ? argv[index + 1] : process.env.QDRAL_RELAY_CONFIG;
  if (path === undefined || path.length === 0) {
    process.stderr.write("usage: node dist/main.js --config <relay.json>\n");
    process.exitCode = 2;
    return;
  }
  const config = loadRelayConfig(path);
  const store = new FileRelayStore(config.stateDir);
  const relay = createRelayServer({
    publicOrigin: config.publicOrigin,
    issuer: config.publicOrigin,
    allowedOrigins: config.allowedOrigins,
    store,
    quotas: config.quotas,
    openaiAppsChallenge: config.openaiAppsChallenge,
    log: (event) => process.stdout.write(`${new Date().toISOString()} ${event}\n`)
  });
  relay.server.listen(config.listenPort, config.listenHost, () => {
    process.stdout.write(`qdral-relay listening on ${config.listenHost}:${config.listenPort} for ${config.publicOrigin}\n`);
  });
  for (const signal of ["SIGINT", "SIGTERM"] as const) {
    process.once(signal, () => {
      relay.server.close();
      relay.server.closeAllConnections();
    });
  }
}

try {
  main(process.argv.slice(2));
} catch (error) {
  process.stderr.write(`qdral-relay: ${error instanceof Error ? error.message : "failed to start"}\n`);
  process.exitCode = 1;
}
