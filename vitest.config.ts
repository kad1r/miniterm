import { defineConfig } from "vitest/config";
import { svelte } from "@sveltejs/vite-plugin-svelte";

export default defineConfig({
  // The svelte plugin lets component tests import .svelte files; they mount
  // them in jsdom (per-file `@vitest-environment jsdom`), which needs the
  // browser build of svelte rather than the server one Node would pick.
  plugins: [svelte()],
  resolve: { conditions: ["browser"] },
  test: { environment: "node", include: ["src/**/*.test.ts"] },
});
