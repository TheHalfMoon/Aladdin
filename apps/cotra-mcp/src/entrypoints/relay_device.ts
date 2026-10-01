import { startRelayTransport } from "../transports/relay_device.js";

/**
 * Relay device entrypoint — reserved for COTRA-P15.
 * Fails closed until the remote uplink grains are canonical.
 */
startRelayTransport();
