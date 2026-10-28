import assert from "node:assert/strict";
import { test as it } from "node:test";
import {
  budgetViolations,
  collectMarkers,
  type Manifest,
  markerViolations,
  startFiles,
} from "./check-bundle.ts";

const KB = 1024;
const BUDGET_KB = 250;
const OVER_BUDGET_BYTES = BUDGET_KB * KB + 1;
const BUDGET_MESSAGE = /250\.0 КБ gzip при бюджете 250 КБ/;
const NO_MARKER_MESSAGE = /session\.ts: нет export const STUB_MARKER/;
const IN_BUNDLE_MESSAGE = /crane-stub:status.*assets\/status-abc\.js/;
const NOT_FOUND_MESSAGE = /не найден в демо-сборке/;

const MANIFEST: Manifest = {
  "index.html": {
    file: "assets/index.js",
    isEntry: true,
    imports: ["_shared.js", "_vendor.js"],
    css: ["assets/index.css"],
  },
  "_shared.js": { file: "assets/shared.js", imports: ["_vendor.js"], css: ["assets/shared.css"] },
  "_vendor.js": { file: "assets/vendor.js" },
  "src/pages/status/ui/StatusPage.tsx": { file: "assets/StatusPage.js", imports: ["_shared.js"] },
};

it("стартовые файлы: entry, статические импорты рекурсивно и их CSS, без повторов", () => {
  // Arrange
  const manifest = MANIFEST;

  // Act
  const files = startFiles(manifest);

  // Assert
  assert.deepEqual(
    files.toSorted((a, b) => a.localeCompare(b)),
    [
      "assets/index.css",
      "assets/index.js",
      "assets/shared.css",
      "assets/shared.js",
      "assets/vendor.js",
    ],
  );
});

it("ленивые страницы (dynamicImports и чанки вне entry) в старт не входят", () => {
  // Arrange
  const manifest: Manifest = {
    ...MANIFEST,
    "index.html": { file: "assets/index.js", isEntry: true, imports: [] },
  };

  // Act
  const files = startFiles(manifest);

  // Assert
  assert.deepEqual(files, ["assets/index.js"]);
});

it("импорт, которого нет в манифесте, не роняет подсчёт", () => {
  // Arrange
  const manifest: Manifest = { a: { file: "a.js", isEntry: true, imports: ["нет-такого"] } };

  // Act
  const files = startFiles(manifest);

  // Assert
  assert.deepEqual(files, ["a.js"]);
});

it("бюджет: ровно 250 КБ — нарушения нет", () => {
  // Arrange
  const bytes = BUDGET_KB * KB;

  // Act
  const violations = budgetViolations(bytes);

  // Assert
  assert.deepEqual(violations, []);
});

it("бюджет: байтом больше 250 КБ — нарушение с размером", () => {
  // Arrange
  const bytes = OVER_BUDGET_BYTES;

  // Act
  const violations = budgetViolations(bytes);

  // Assert
  assert.equal(violations.length, 1);
  assert.match(violations[0] ?? "", BUDGET_MESSAGE);
});

it("маркеры собираются из модулей данных", () => {
  // Arrange
  const modules = [
    { path: "status.ts", content: 'export const STUB_MARKER = "crane-stub:status";' },
    {
      path: "me.ts",
      content: 'export const STUB_MARKER = "crane-stub:me";\nexport function getMe() {}',
    },
  ];

  // Act
  const result = collectMarkers(modules);

  // Assert
  assert.deepEqual(result, { markers: ["crane-stub:status", "crane-stub:me"], violations: [] });
});

it("модуль данных без маркера — нарушение", () => {
  // Arrange
  const modules = [{ path: "session.ts", content: "export function getSession() {}" }];

  // Act
  const result = collectMarkers(modules);

  // Assert
  assert.equal(result.violations.length, 1);
  assert.match(result.violations[0] ?? "", NO_MARKER_MESSAGE);
});

it("без демо: маркер в dist — нарушение с именем файла", () => {
  // Arrange
  const bundle = [
    { path: "assets/index.js", content: "ничего" },
    { path: "assets/status-abc.js", content: 'x="crane-stub:status"' },
  ];

  // Act
  const violations = markerViolations(["crane-stub:status"], bundle, false);

  // Assert
  assert.equal(violations.length, 1);
  assert.match(violations[0] ?? "", IN_BUNDLE_MESSAGE);
});

it("без демо: чистый dist — всё в порядке", () => {
  // Arrange
  const bundle = [{ path: "assets/index.js", content: "ничего" }];

  // Act
  const violations = markerViolations(["crane-stub:status"], bundle, false);

  // Assert
  assert.deepEqual(violations, []);
});

it("демо: маркер найден — всё в порядке", () => {
  // Arrange
  const bundle = [{ path: "assets/status.js", content: 'x="crane-stub:status"' }];

  // Act
  const violations = markerViolations(["crane-stub:status"], bundle, true);

  // Assert
  assert.deepEqual(violations, []);
});

it("демо: маркера нет — нарушение: флаг не сработал", () => {
  // Arrange
  const bundle = [{ path: "assets/index.js", content: "ничего" }];

  // Act
  const violations = markerViolations(["crane-stub:status"], bundle, true);

  // Assert
  assert.equal(violations.length, 1);
  assert.match(violations[0] ?? "", NOT_FOUND_MESSAGE);
});
