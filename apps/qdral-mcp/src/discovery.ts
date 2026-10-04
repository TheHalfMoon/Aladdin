import type { KernelClient } from "./kernel.js";
import { buildQdralServer, type TransportContext } from "./server.js";

/**
 * SG-000067 in-process tool discovery through the authoritative MCP builder.
 * It answers `tools/list` exactly as a client on the given transport sees
 * it, without spawning qdrald: the discovery kernel throws if any tool is
 * called. Packaging and review evidence use it so provider-facing tool
 * metadata is never written by hand.
 */
export interface DiscoveredTool {
  readonly name: string;
  readonly title?: string;
  readonly description?: string;
  readonly inputSchema?: unknown;
  readonly annotations?: Record<string, unknown>;
  readonly _meta?: Record<string, unknown>;
}

type Message = { jsonrpc: "2.0"; id?: number; method?: string; params?: unknown; result?: unknown };

class DiscoveryTransport {
  onmessage?: (message: Message) => void;
  onclose?: () => void;
  onerror?: (error: Error) => void;
  private readonly pending = new Map<number, (message: Message) => void>();
  async start(): Promise<void> {}
  async send(message: Message): Promise<void> {
    if (message.id !== undefined && message.method === undefined) {
      this.pending.get(message.id)?.(message);
      this.pending.delete(message.id);
    }
  }
  async close(): Promise<void> {
    this.onclose?.();
  }
  request(id: number, method: string, params: unknown): Promise<Message> {
    return new Promise((resolve) => {
      this.pending.set(id, resolve);
      this.onmessage?.({ jsonrpc: "2.0", id, method, params });
    });
  }
  notify(method: string): void {
    this.onmessage?.({ jsonrpc: "2.0", method });
  }
}

function discoveryKernel(): KernelClient {
  return {
    sessionId: "discovery",
    call: async () => {
      throw new Error("discovery never calls the kernel");
    },
    close: () => {},
    pending: new Map(),
    child: {} as never
  } as unknown as KernelClient;
}

export async function discoverTools(context: TransportContext): Promise<DiscoveredTool[]> {
  const server = buildQdralServer(discoveryKernel(), "default", context);
  const transport = new DiscoveryTransport();
  await server.connect(transport as never);
  await transport.request(1, "initialize", {
    protocolVersion: "2025-06-18",
    capabilities: {},
    clientInfo: { name: "qdral-discovery", version: "1" }
  });
  transport.notify("notifications/initialized");
  const listed = await transport.request(2, "tools/list", {});
  await server.close();
  return (listed.result as { tools: DiscoveredTool[] }).tools;
}
