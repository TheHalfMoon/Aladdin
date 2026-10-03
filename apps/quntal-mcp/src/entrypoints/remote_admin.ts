/**
 * SG-000057 local remote enrollment entrypoint, launched only by
 * `quntal remote enable` and `quntal remote pair` after STRONG presence.
 *
 *   remote_admin.js enable --relay <origin> [--workspace <id>]
 *   remote_admin.js pair
 *
 * Reads QUNTAL_DEVICE_KEY_PATH and QUNTAL_UPLINK_CONFIG. Never prints the
 * private key; the pairing code is shown once on this console.
 */
import { createInterface } from "node:readline/promises";
import { enableRemote, pairRemote } from "../remote_enrollment.js";

function required(name: string): string {
  const value = process.env[name];
  if (value === undefined || value.length === 0) {
    throw new Error(`PAIRING_REQUIRED: ${name} is not configured`);
  }
  return value;
}

function option(args: readonly string[], name: string): string | undefined {
  const index = args.indexOf(name);
  return index >= 0 ? args[index + 1] : undefined;
}

async function main(args: readonly string[]): Promise<void> {
  const deviceKeyPath = required("QUNTAL_DEVICE_KEY_PATH");
  const uplinkPath = required("QUNTAL_UPLINK_CONFIG");
  const fetchImpl = (url: string, init: RequestInit) => fetch(url, init);
  if (args[0] === "enable") {
    const relayOrigin = option(args, "--relay");
    if (relayOrigin === undefined) {
      throw new Error("PAIRING_REQUIRED: --relay <origin> is required");
    }
    const result = await enableRemote({
      relayOrigin,
      defaultWorkspace: option(args, "--workspace") ?? process.env.QUNTAL_DEFAULT_WORKSPACE ?? "default",
      deviceKeyPath,
      uplinkPath,
      fetch: fetchImpl,
      nowMs: Date.now()
    });
    process.stdout.write(
      `Remote access enabled for device ${result.deviceId}${result.created ? " (new device key)" : ""}.\n` +
        "Next: connect Quntal in your AI client, then run `quntal remote pair` when it asks for a pairing code.\n"
    );
    return;
  }
  if (args[0] === "pair") {
    const console = createInterface({ input: process.stdin, output: process.stdout });
    try {
      const paired = await pairRemote({
        deviceKeyPath,
        uplinkPath,
        fetch: fetchImpl,
        now: Date.now,
        sleep: (ms) => new Promise((resolve) => setTimeout(resolve, ms)),
        display: (code, expiresAtMs) => {
          const minutes = Math.max(1, Math.round((expiresAtMs - Date.now()) / 60_000));
          process.stdout.write(`\nPairing code (valid ${minutes} minutes, single use):\n\n  ${code}\n\nEnter it on the authorization page in your AI client.\n`);
        },
        confirm: async (request) => {
          process.stdout.write(
            `\nApprove this remote connection?\n  client:   ${request.clientName} (${request.clientId})\n  returns:  ${request.redirectOrigin}\n  scopes:   ${request.scopes.join(" ")}\n  route:    ${request.remoteConnectionId}\n` +
              "This links the client to this computer. It grants no workspace trust and no access until you also run `quntal remote allow`.\n"
          );
          const answer = await console.question("Type yes to approve: ");
          return answer.trim().toLowerCase() === "yes";
        }
      });
      if (paired === null) {
        process.stdout.write("Pairing was not completed.\n");
        process.exitCode = 1;
        return;
      }
      process.stdout.write(
        `Paired ${paired.remoteConnectionId}.\nNext: run \`quntal remote connect\`, then \`quntal remote allow --connection ${paired.remoteConnectionId}\` to allow a finite session.\n`
      );
    } finally {
      console.close();
    }
    return;
  }
  throw new Error("PAIRING_REQUIRED: usage: remote_admin.js enable --relay <origin> | pair");
}

main(process.argv.slice(2)).catch((error: unknown) => {
  process.stderr.write(`[quntal-remote] ${error instanceof Error ? error.message : "failed"}\n`);
  process.exitCode = 1;
});
