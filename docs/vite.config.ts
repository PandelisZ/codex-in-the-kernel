import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { defineConfig } from "vite";

const rootDir = dirname(fileURLToPath(import.meta.url));

export default defineConfig({
  root: rootDir,
  appType: "mpa",
  base: process.env.PAGES_BASE ?? "/",
  build: {
    emptyOutDir: true,
    outDir: resolve(rootDir, "../dist/docs-site"),
    rollupOptions: {
      input: {
        home: resolve(rootDir, "index.html"),
        research: resolve(rootDir, "research/index.html"),
      },
    },
  },
});
