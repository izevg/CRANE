// Шаг после `vite build` (вызывается из `build` в web/package.json): бюджет первой загрузки и защита от демо-данных.
// 1) Стартовый чанк (entry и статически связанные чанки с их CSS) не больше 250 КБ gzip, иначе выход с кодом 1.
// 2) Без CRANE_UI_DEMO=1 в манифесте нет ни одного модуля данных заглушек (`src/shared/api/stubs/data/`).
//    С CRANE_UI_DEMO=1 наоборот: модули обязаны быть, иначе флаг не сработал.
// Использование: node ../tools/check-bundle.ts [dist] (из web/).
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { gzipSync } from "node:zlib";

const BYTES_PER_KB = 1024;
export const BUDGET_KB = 250;
const STUB_DATA_PREFIX = "src/shared/api/stubs/data/";
const DEMO_VALUE = "1";

export interface ManifestChunk {
  file: string;
  isEntry?: boolean;
  /** Статические импорты: ключи манифеста. Динамические (`dynamicImports`) в старт не входят. */
  imports?: string[];
  css?: string[];
}

export type Manifest = Record<string, ManifestChunk>;

/** Файлы стартовой загрузки: entry и всё, что подключено к нему статически (JS и CSS), без повторов. */
export function startFiles(manifest: Manifest): string[] {
  const files = new Set<string>();
  const visited = new Set<string>();
  const visit = (key: string): void => {
    const chunk = manifest[key];
    if (chunk === undefined || visited.has(key)) {
      return;
    }
    visited.add(key);
    files.add(chunk.file);
    for (const css of chunk.css ?? []) {
      files.add(css);
    }
    for (const imported of chunk.imports ?? []) {
      visit(imported);
    }
  };
  for (const [key, chunk] of Object.entries(manifest)) {
    if (chunk.isEntry === true) {
      visit(key);
    }
  }
  return [...files];
}

/** Нарушение бюджета: общий размер стартовых файлов (gzip, байты) больше лимита. */
export function budgetViolations(gzipBytes: number, limitKb: number = BUDGET_KB): string[] {
  const limit = limitKb * BYTES_PER_KB;
  if (gzipBytes <= limit) {
    return [];
  }
  const actualKb = (gzipBytes / BYTES_PER_KB).toFixed(1);
  return [`стартовый чанк ${actualKb} КБ gzip при бюджете ${limitKb} КБ`];
}

/** Ключи манифеста, которые являются модулями данных заглушек. */
export function stubModuleKeys(manifest: Manifest): string[] {
  return Object.keys(manifest).filter((key) => key.startsWith(STUB_DATA_PREFIX));
}

/**
 * Демо выключено: ни одного модуля данных заглушек в сборке. Демо включено: хотя бы один обязан быть, иначе
 * сборка с флагом молча осталась без заглушек. Статический импорт, встроивший данные в обычный чанк, манифест
 * не покажет: от него защищает правило dependency-cruiser.
 */
export function stubViolations(manifest: Manifest, demo: boolean): string[] {
  const keys = stubModuleKeys(manifest);
  if (!demo && keys.length > 0) {
    return [`модули данных заглушек в сборке без демо: ${keys.join(", ")}`];
  }
  if (demo && keys.length === 0) {
    return ["модулей данных заглушек нет в демо-сборке: CRANE_UI_DEMO не сработал"];
  }
  return [];
}

function main(): void {
  const dist = process.argv[2] ?? "dist";
  const { CRANE_UI_DEMO } = process.env;
  const demo = CRANE_UI_DEMO === DEMO_VALUE;

  const manifest = JSON.parse(
    readFileSync(join(dist, ".vite", "manifest.json"), "utf8"),
  ) as Manifest;
  const start = startFiles(manifest);
  const gzipBytes = start.reduce(
    (sum, file) => sum + gzipSync(readFileSync(join(dist, file))).length,
    0,
  );
  const kb = (gzipBytes / BYTES_PER_KB).toFixed(1);
  process.stdout.write(
    `check-bundle: стартовых файлов ${start.length}, ${kb} КБ gzip (бюджет ${BUDGET_KB} КБ)\n`,
  );

  const violations = [...budgetViolations(gzipBytes), ...stubViolations(manifest, demo)];
  const mode = demo ? "демо-сборка" : "сборка без демо";
  process.stdout.write(
    `check-bundle: ${mode}, модулей данных заглушек: ${stubModuleKeys(manifest).length}\n`,
  );
  if (violations.length > 0) {
    for (const violation of violations) {
      process.stderr.write(`check-bundle: ${violation}\n`);
    }
    process.exit(1);
  }
}

if (import.meta.main) {
  main();
}
