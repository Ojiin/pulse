// The dev server for the film: the repo is its root, so film.html (here) and
// the panel (desktop/ui) share one origin; the panel's own packages come from
// desktop/ui/node_modules. Started by render.mjs.
import { fileURLToPath } from 'node:url'

const repo = fileURLToPath(new URL('../..', import.meta.url))

export default {
  root: repo,
  cacheDir: `${repo}desktop/ui/node_modules/.vite-video`,
  logLevel: 'warn',
  esbuild: { jsx: 'automatic' },
  optimizeDeps: { entries: ['desktop/ui/src/main.tsx'] },
  server: { port: 5288, strictPort: true, fs: { allow: [repo] } },
}
