// Шаг после `vite build` (вызывается из `build` в web/package.json): бюджет первой загрузки и защита от демо-данных.
// 1) Стартовый чанк (entry и статически связанные чанки с их CSS) не больше 250 КБ gzip, иначе выход с кодом 1.
// 2) Без CRANE_UI_DEMO=1 в dist нет ни одного модуля заглушек: каждый модуль данных экспортирует STUB_MARKER, и
//    маркер ищется по всему dist. С CRANE_UI_DEMO=1 наоборот: маркеры обязаны быть, иначе флаг не сработал.
// Использование: node ../tools/check-bundle.ts [dist] [каталог модулей заглушек] (из web/).
import { readdirSync, readFileSync } from "node:fs";
import { join, relative } from "node:path";
import { gzipSync } from "node:zlib";

const BYTES_PER_KB = 1024;
export const BUDGET_KB = 250;
const MARKER_EXPORT = /export const STUB_MARKER\s*=\s*"([^"]+)"/;
const DEMO_VALUE = "1";
const BUNDLE_TEXT = /\.(js|css|html|json)$/;

export interface ManifestChunk {
  file: string;
  isEntry?: boolean;
  /** Статические импорты: ключи манифеста. Динамические (`dynamicImports`) в старт не входят. */
  imports?: string[];
  css?: string[];
}

export type Manifest = Record<string, ManifestChunk>;

export interface SourceFile {
  path: string;
  content: string;
}

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

/** Маркеры модулей данных заглушек; модуль без маркера — нарушение, иначе его нечем найти в dist. */
export function collectMarkers(modules: SourceFile[]): { markers: string[]; violations: string[] } {
  const markers: string[] = [];
  const violations: string[] = [];
  for (const module of modules) {
    const marker = MARKER_EXPORT.exec(module.content)?.[1];
    if (marker === undefined) {
      violations.push(`${module.path}: нет export const STUB_MARKER = "…"`);
    } else {
      markers.push(marker);
    }
  }
  return { markers, violations };
}

/**
 * Демо выключено: ни один маркер не должен встретиться в dist. Демо включено: каждый маркер обязан встретиться,
 * иначе сборка с флагом молча осталась без заглушек.
 */
export function markerViolations(markers: string[], bundle: SourceFile[], demo: boolean): string[] {
  const violations: string[] = [];
  for (const marker of markers) {
    const where = bundle.filter((file) => file.content.includes(marker)).map((file) => file.path);
    if (!demo && where.length > 0) {
      violations.push(`маркер ${marker} в сборке без демо: ${where.join(", ")}`);
    }
    if (demo && where.length === 0) {
      violations.push(`маркер ${marker} не найден в демо-сборке: CRANE_UI_DEMO не сработал`);
    }
  }
  return violations;
}

function listFiles(dir: string): string[] {
  return readdirSync(dir, { withFileTypes: true }).flatMap((entry) =>
    entry.isDirectory() ? listFiles(join(dir, entry.name)) : [join(dir, entry.name)],
  );
}

function readAll(root: string, files: string[]): SourceFile[] {
  return files.map((path) => ({ path: relative(root, path), content: readFileSync(path, "utf8") }));
}

function main(): void {
  const dist = process.argv[2] ?? "dist";
  const stubsDir = process.argv[3] ?? "src/shared/api/stubs/data";
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

  const modules = readAll(
    stubsDir,
    listFiles(stubsDir).filter((path) => path.endsWith(".ts")),
  );
  const { markers, violations: unmarked } = collectMarkers(modules);
  const bundle = readAll(
    dist,
    listFiles(dist).filter((path) => BUNDLE_TEXT.test(path)),
  );
  const violations = [
    ...budgetViolations(gzipBytes),
    ...unmarked,
    ...markerViolations(markers, bundle, demo),
  ];

  const mode = demo ? "демо-сборка" : "сборка без демо";
  process.stdout.write(`check-bundle: ${mode}, маркеров заглушек: ${markers.length}\n`);
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
