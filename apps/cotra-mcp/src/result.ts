import type { KernelResponse } from "./kernel.js";

/**
 * Project a kernel response into an MCP tool result.
 *
 * This is the single authoritative result projection for the local MCP edge.
 * Transport adapters must reuse this function and must not define an
 * independent projection that could add authority or leak internal metadata.
 *
 * The shape is byte-compatible with the Cotra v0.1 public MCP surface:
 * failures return `isError: true` with a JSON error envelope, successes
 * return the kernel result plus bounded evidence. No transport identity,
 * device identity, token material, or approval secrets are added here.
 */
export function projectKernelResult(response: KernelResponse) {
  if (!response.ok) {
    return {
      isError: true,
      content: [
        {
          type: "text" as const,
          text: JSON.stringify(
            {
              error: response.error ?? {
                code: "INTERNAL_ERROR",
                message: "cotrad returned an unspecified failure"
              }
            },
            null,
            2
          )
        }
      ]
    };
  }

  return {
    content: [
      {
        type: "text" as const,
        text: JSON.stringify(
          {
            result: response.result,
            evidence: response.evidence
          },
          null,
          2
        )
      }
    ]
  };
}
