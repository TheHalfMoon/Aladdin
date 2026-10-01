/**
 * Loopback Streamable HTTP transport — reserved skeleton for SG-000050.
 *
 * SG-000048 establishes the transport-neutral structure only. No HTTP
 * listener, no LAN exposure, no credential minting, and no tool authority
 * are introduced here. Any call fails closed with TRANSPORT_UNAVAILABLE
 * until SG-000050 implements loopback-only bind, Host/Origin validation,
 * DNS-rebinding protection, per-user credential, and bounded limits.
 */
export const LOOPBACK_TRANSPORT_RESERVED = "SG-000050";

export function startLoopbackTransport(): never {
  throw new Error(
    "TRANSPORT_UNAVAILABLE: loopback Streamable HTTP is reserved for SG-000050 and is not yet implemented"
  );
}
