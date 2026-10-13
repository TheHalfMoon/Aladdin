// SG-000108 A8: TesterArmy E2E evidence. The web engine attaches over CDP to
// a fresh, installed Edge per attempt (no browser download), confined to the
// controlled fixture origin: fixture.test resolves to loopback and every
// other name to nothing, and only the fixture's own certificate is trusted.
// No model is configured, so no step can call one.
import type { E2EConfig } from 'e2e';
import { web } from '@e2e-dev/web';
import { freshEdge } from './edge.ts';

export default {
  targets: [
    {
      name: 'fixture',
      engine: web({ connect: { cdpEndpoint: (signal) => freshEdge(signal) } }),
      app: { url: 'https://fixture.test' },
    },
  ],
  workers: 1,
  retries: 0,
} satisfies E2EConfig;
