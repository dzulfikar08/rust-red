import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import { fileURLToPath, URL } from "node:url";

export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: {
      "@": fileURLToPath(new URL("./src", import.meta.url)),
    },
  },
  server: {
    port: 3000,
    proxy: {
      "/flows": {
        target: "http://127.0.0.1:1888",
        changeOrigin: true,
      },
      "/flow": {
        target: "http://127.0.0.1:1888",
        changeOrigin: true,
      },
      "/nodes": {
        target: "http://127.0.0.1:1888",
        changeOrigin: true,
      },
      "/settings": {
        target: "http://127.0.0.1:1888",
        changeOrigin: true,
      },
      "/comms": {
        target: "ws://127.0.0.1:1888",
        ws: true,
        changeOrigin: true,
      },
      "/context": {
        target: "http://127.0.0.1:1888",
        changeOrigin: true,
      },
      "/library": {
        target: "http://127.0.0.1:1888",
        changeOrigin: true,
      },
      "/api": {
        target: "http://127.0.0.1:1888",
        changeOrigin: true,
      },
    },
  },
});
