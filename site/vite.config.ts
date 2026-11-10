import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'

// Served from https://c2-poly.github.io/voix-fluide/
export default defineConfig({
  base: '/voix-fluide/',
  plugins: [react(), tailwindcss()],
})
