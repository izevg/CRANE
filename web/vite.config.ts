import { existsSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

/**
 * Ближайший каталог `name` вверх от `from`. Stryker копирует web/ в `.stryker-tmp/sandbox-*`, и там `../brand` уже
 * не корневой brand/: ищем вверх, а не считаем фиксированный путь, и логотип не копируем.
 */
function findUp(from: string, name: string): string {
  let dir = from;
  while (!existsSync(join(dir, name))) {
    const parent = dirname(dir);
    if (parent === dir) {
      throw new Error(`Каталог ${name}/ не найден выше ${from}`);
    }
    dir = parent;
  }
  return join(dir, name);
}

const here = dirname(fileURLToPath(import.meta.url));

// Сборка и Storybook. Тесты — в vitest.config.ts, чтобы story-проект не попадал в production-конфиг.
export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: {
      "~test": new URL("./test", import.meta.url).pathname,
      // Логотип CRANE живёт в корневом brand/ (единственный источник истины), в web/ его не копируем.
      "~brand": findUp(here, "brand"),
      "~": new URL("./src", import.meta.url).pathname,
    },
  },
});
