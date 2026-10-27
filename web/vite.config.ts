import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

// Сборка и Storybook. Тесты — в vitest.config.ts, чтобы story-проект не попадал в production-конфиг.
export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: {
      "~test": new URL("./test", import.meta.url).pathname,
      // Логотип CRANE живёт в корневом brand/ (единственный источник истины), в web/ его не копируем.
      "~brand": new URL("../brand", import.meta.url).pathname,
      "~": new URL("./src", import.meta.url).pathname,
    },
  },
});
