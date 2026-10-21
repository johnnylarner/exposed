import { defineConfig } from "vite";
import { sveltekit } from "@sveltejs/kit/vite";
import { mdsvex } from "mdsvex";
import adapter from "@sveltejs/adapter-node";
export default defineConfig({
  plugins: [
    sveltekit({
      adapter: adapter(),
      extensions: [".svelte", ".md"],
      preprocess: mdsvex({ extensions: [".md"], smartypants: false }),
    }),
  ],
  server: {
    host: "127.0.0.1",
    port: 5173,
    strictPort: true,
    watch: { usePolling: process.env.EXPOSED_WATCH_POLL === "true" },
  },
  preview: { host: "127.0.0.1", port: 4173, strictPort: true },
});
