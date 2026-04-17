import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { defineConfig } from "vite";

const rootDir = dirname(fileURLToPath(import.meta.url));

export default defineConfig({
  root: rootDir,
  base: process.env.PAGES_BASE ?? "/",
  build: {
    emptyOutDir: true,
    outDir: resolve(rootDir, "../dist/docs-site"),
  },
});
