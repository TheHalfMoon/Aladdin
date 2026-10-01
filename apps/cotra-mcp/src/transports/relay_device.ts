/**
 * Relay device transport — reserved skeleton for COTRA-P15.
 *
 * SG-000048 establishes the transport-neutral structure only. No relay
 * connection, no outbound channel, no device-key use, no OAuth handling,
 * and no tool authority are introduced here. Any call fails closed with
 * TRANSPORT_UNAVAILABLE until the P15 device identity, pairing, OAuth,
 * and outbound-only uplink grains are canonical.
 */
export const RELAY_TRANSPORT_RESERVED = "COTRA-P15";

export function startRelayTransport(): never {
  throw new Error(
    "TRANSPORT_UNAVAILABLE: relay device transport is reserved for COTRA-P15 and is not yet implemented"
  );
}
