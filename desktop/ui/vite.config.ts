import react from '@vitejs/plugin-react'
import { defineConfig } from 'vite'

// The page of the desktop app (../src-tauri): `tauri build` bundles dist,
// `tauri dev` loads this dev server with hot reload.
export default defineConfig({
  plugins: [react()],
  base: './',
  build: { outDir: 'dist', emptyOutDir: true },
  // the app icons in ../assets are shown in the foot of the panel
  server: { port: 5199, strictPort: true, fs: { allow: ['..'] } },
})
