import { fileURLToPath } from "node:url";
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { htmlCspPlugin } from "@tab/vite-config/csp";

const csp: Record<string, string> = {
  "default-src": "'self'",
  "script-src": "'self'",
  "img-src": "'self'",
  "style-src": "'self'",
  "connect-src": "'self' api:",
  "object-src": "'none'",
  "frame-src": "'none'",
};

// https://vitejs.dev/config/
export default defineConfig(({ command }) => ({
  plugins: [react(), htmlCspPlugin(csp)],
  server: {
    port: 3433,
  },
  build: {
    target: command === "build" ? "es2017" : "esnext",
    sourcemap: command !== "build",
  },
  esbuild: {
    target: command === "build" ? ["es2017", "safari11"] : undefined,
  },
  resolve: {
    alias: {
      "@": fileURLToPath(new URL("./src", import.meta.url)),
      "@assets": fileURLToPath(new URL("./assets", import.meta.url)),
    },
  },
}));
