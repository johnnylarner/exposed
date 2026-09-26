import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

const proxy = {
  "/api": {
    target: process.env.EXPOSED_API_ORIGIN || "http://127.0.0.1:6999",
    changeOrigin: true,
    rewrite: (path: string) => path.replace(/^\/api/, ""),
  },
};

export default defineConfig({
  plugins: [svelte()],
  server: {
    host: "127.0.0.1",
    port: 5173,
    strictPort: true,
    proxy,
    watch: { usePolling: process.env.EXPOSED_WATCH_POLL === "true" },
  },
  preview: { host: "127.0.0.1", port: 4173, strictPort: true, proxy },
});
