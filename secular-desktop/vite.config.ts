// secular-desktop/vite.config.ts
import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

export default defineConfig({
  plugins: [react()],
  // Secular: prevent Vite from obscuring Rust errors
  clearScreen: false,
  // Secular: tauri expects a fixed port, fail if that port is not available
  server: {
    port: 5173,
    strictPort: true,
  },
  // Secular: make sure the Environment Variables are available
  envPrefix: ['VITE_', 'TAURI_'],
  build: {
    // Tauri uses Chromium on mobile and WebKit on desktop
    target: 'es2021',
    // don't minify for debug builds
    minify: process.env.TAURI_DEBUG === 'false' ? 'esbuild' : false,
    // produce sourcemaps for debug builds
    sourcemap: !!process.env.TAURI_DEBUG,
  },
})
