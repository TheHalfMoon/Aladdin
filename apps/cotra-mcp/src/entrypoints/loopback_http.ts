import { startLoopbackTransport } from "../transports/loopback_http.js";

/**
 * Loopback HTTP entrypoint — reserved for SG-000050.
 * Fails closed until the loopback grain is canonical.
 */
startLoopbackTransport();
