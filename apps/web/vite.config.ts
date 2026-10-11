import { readFileSync } from 'node:fs';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig, type ServerOptions } from 'vite';

// The Compose development stack serves Vite behind the suite gateway at the
// installation's public origin, including through a tunnel.
function developmentServer(): ServerOptions | undefined {
  if (!process.env.LOOSKI_DEV_PUBLIC_URL) return undefined;
  const origin = new URL(process.env.LOOSKI_DEV_PUBLIC_URL);
  const secure = origin.protocol === 'https:';
  const tls = process.env.LOOSKI_DEV_TLS_DIR;
  return {
    allowedHosts: [origin.hostname],
    https: secure
      ? {
          key: readFileSync(`${tls}/key.pem`),
          cert: readFileSync(`${tls}/cert.pem`),
        }
      : undefined,
    hmr: {
      protocol: secure ? 'wss' : 'ws',
      host: origin.hostname,
      clientPort: Number(origin.port) || 443,
    },
    // File events from macOS VM bind mounts are unreliable.
    watch: { usePolling: true, interval: 300 },
  };
}

export default defineConfig({
  plugins: [sveltekit()],
  server: developmentServer(),
});
