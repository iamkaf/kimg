import { fileURLToPath } from "url";
import { resolve } from "path";
import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import wasm from "vite-plugin-wasm";
import topLevelAwait from "vite-plugin-top-level-await";

const __dirname = fileURLToPath(new URL(".", import.meta.url));

export default defineConfig({
  root: __dirname,
  plugins: [wasm(), topLevelAwait(), svelte()],
  resolve: {
    alias: {
      "#kimg": resolve(__dirname, "../dist"),
    },
  },
  server: {
    fs: {
      allow: [resolve(__dirname, "..")],
    },
  },
  optimizeDeps: {
    exclude: ["../dist/index.js", "../dist/raw.js"],
  },
  build: {
    outDir: resolve(__dirname, "dist"),
    emptyOutDir: true,
  },
});
