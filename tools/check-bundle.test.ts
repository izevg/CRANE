import assert from "node:assert/strict";
import { test as it } from "node:test";
import {
  budgetViolations,
  type Manifest,
  startFiles,
  stubModuleKeys,
  stubViolations,
} from "./check-bundle.ts";

const KB = 1024;
const BUDGET_KB = 250;
const OVER_BUDGET_BYTES = BUDGET_KB * KB + 1;
const BUDGET_MESSAGE = /250\.0 КБ gzip при бюджете 250 КБ/;
const IN_MANIFEST_MESSAGE = /в сборке без демо: src\/shared\/api\/stubs\/data\/status\.ts/;
const NOT_FOUND_MESSAGE = /нет в демо-сборке/;

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

const STUB_KEY = "src/shared/api/stubs/data/status.ts";
const WITH_STUBS: Manifest = {
  ...MANIFEST,
  [STUB_KEY]: { file: "assets/status-abc.js" },
  "src/shared/api/stubs/data/me.ts": { file: "assets/me-abc.js" },
};

it("модули данных заглушек распознаются по префиксу ключа манифеста", () => {
  // Arrange
  const manifest = WITH_STUBS;

  // Act
  const keys = stubModuleKeys(manifest);

  // Assert
  assert.deepEqual(keys, [STUB_KEY, "src/shared/api/stubs/data/me.ts"]);
});

it("страница модулем данных не считается", () => {
  // Arrange
  const manifest: Manifest = {
    "src/pages/status/ui/StatusPage.tsx": { file: "assets/StatusPage.js" },
  };

  // Act
  const keys = stubModuleKeys(manifest);

  // Assert
  assert.deepEqual(keys, []);
});

it("без демо: модуль данных в манифесте — нарушение с его ключами", () => {
  // Arrange
  const manifest = WITH_STUBS;

  // Act
  const violations = stubViolations(manifest, false);

  // Assert
  assert.equal(violations.length, 1);
  assert.match(violations[0] ?? "", IN_MANIFEST_MESSAGE);
});

it("без демо: чистый манифест — всё в порядке", () => {
  // Arrange
  const manifest = MANIFEST;

  // Act
  const violations = stubViolations(manifest, false);

  // Assert
  assert.deepEqual(violations, []);
});

it("демо: модули данных есть — всё в порядке", () => {
  // Arrange
  const manifest = WITH_STUBS;

  // Act
  const violations = stubViolations(manifest, true);

  // Assert
  assert.deepEqual(violations, []);
});

it("демо: модулей данных нет — нарушение: флаг не сработал", () => {
  // Arrange
  const manifest = MANIFEST;

  // Act
  const violations = stubViolations(manifest, true);

  // Assert
  assert.equal(violations.length, 1);
  assert.match(violations[0] ?? "", NOT_FOUND_MESSAGE);
});
