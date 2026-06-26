import { defineConfig } from "vitest/config";

// Tauri expects a fixed dev-server port and ignores src-tauri changes.
export default defineConfig({
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**"] },
  },
  build: {
    rollupOptions: {
      input: {
        main: "index.html",
        settings: "settings.html",
      },
    },
  },
  test: {
    // animationForMood is a pure function with no DOM dependency.
    environment: "node",
    include: ["src/**/*.test.ts"],
  },
});
