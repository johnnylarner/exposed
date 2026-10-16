import { defineConfig } from "vite";
import { sveltekit } from "@sveltejs/kit/vite";
import adapter from "@sveltejs/adapter-node";
export default defineConfig({
  plugins: [sveltekit({ adapter: adapter() })],
  server: {
    host: "127.0.0.1",
    port: 5173,
    strictPort: true,
    watch: { usePolling: process.env.EXPOSED_WATCH_POLL === "true" },
  },
  preview: { host: "127.0.0.1", port: 4173, strictPort: true },
});
