import { existsSync, realpathSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { lingui } from "@lingui/vite-plugin";
import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig, type Plugin, searchForWorkspaceRoot } from "vite";

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

/**
 * Orca делает `node_modules` и `web/node_modules` символическими ссылками на основной каталог (`orca.yaml`,
 * `worktree.sharedDirectories`). Vite отдаёт файлы (в том числе в browser-режиме Vitest и в Storybook) только из
 * разрешённых корней, а реальный путь ссылки лежит вне worktree: `setup-file.js` аддона Storybook не загружался, и
 * падали все story-тесты. Явный `server.fs.allow` заменяет значение по умолчанию, поэтому корень рабочей области
 * возвращаем сами. Без символических ссылок реальные пути совпадают с обычными и ничего не меняется.
 */
export const FS_ALLOW = [
  searchForWorkspaceRoot(process.cwd()),
  ...[join(here, "node_modules"), join(here, "..", "node_modules")]
    .filter((path) => existsSync(path))
    .map((path) => realpathSync(path)),
];

const DEMO_DEFINE = "__CRANE_UI_DEMO__";

/**
 * Демо-режим интерфейса (заглушки API) — константа сборки: true в `vite` dev, Storybook dev и тестах (команда serve),
 * а в сборке только при `CRANE_UI_DEMO=1` (его же читает tools/check-bundle.ts; статической витрине флаг задан в
 * скрипте `build-storybook`). Единственный флаг: окружение сборщика с лишней переменной демо не включит. Плагин, а не функция-конфиг: `vitest*.config.ts` сливают этот конфиг как объект.
 */
function demoFlag(): Plugin {
  return {
    name: "crane-demo-flag",
    config(_config, { command }) {
      const { CRANE_UI_DEMO } = process.env;
      const demo = command === "serve" || CRANE_UI_DEMO === "1";
      return { define: { [DEMO_DEFINE]: JSON.stringify(demo) } };
    },
  };
}

// Сборка и Storybook. Тесты — в vitest.config.ts, чтобы story-проект не попадал в production-конфиг.
export default defineConfig({
  // failOnMissing: сборка падает, если у сообщения нет перевода в каталоге (EN не должен молча показывать русский)
  plugins: [
    react(),
    lingui({ macroTransform: true, failOnMissing: true, failOnCompileError: true }),
    tailwindcss(),
    demoFlag(),
  ],
  // manifest нужен шагу после сборки (tools/check-bundle.ts): по нему считается стартовый чанк
  build: { manifest: true },
  server: { fs: { allow: FS_ALLOW } },
  resolve: {
    alias: {
      "~test": new URL("./test", import.meta.url).pathname,
      // Логотип CRANE живёт в корневом brand/ (единственный источник истины), в web/ его не копируем.
      "~brand": findUp(here, "brand"),
      "~": new URL("./src", import.meta.url).pathname,
    },
  },
});
