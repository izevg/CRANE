import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: {
      "~test": new URL("./test", import.meta.url).pathname,
      "~": new URL("./src", import.meta.url).pathname,
    },
  },
  test: {
    environment: "jsdom",
    include: ["src/**/*.test.{ts,tsx}"],
    setupFiles: ["./test/setup.ts"],
    // CSS в тестах не обрабатывается; исключение — токены движения, которые сверяет тест с TS-константами
    css: { include: [/motion\.css/] },
  },
});
