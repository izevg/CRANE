import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { after, test as it } from "node:test";
import { fileURLToPath, pathToFileURL } from "node:url";

// Негативные тесты правил web/.dependency-cruiser.ts про границу shared/api: на маленьком дереве-фикстуре
// в каталоге ОС запускается настоящий `cruise` с настоящим набором правил. Заглушки и транспорт не должны
// протекать в экраны, а модули заглушек зависят только от shared/api и shared/lib.

const web = resolve(dirname(fileURLToPath(import.meta.url)), "../../web");
const INTERNALS = "api-internals-are-private";
const STUBS_ONLY = "api-stubs-import-only-api-and-lib";

interface Violation {
  rule: string;
  from: string;
  to: string;
}

interface Cruiser {
  cruise: (
    files: string[],
    options: Record<string, unknown>,
  ) => Promise<{
    output: { summary: { violations: { rule: { name: string }; from: string; to: string }[] } };
  }>;
}

const FIXTURE: Record<string, string> = {
  "src/shared/api/index.ts": 'export { fetcher } from "./http/fetcher";\nexport const api = 1;\n',
  "src/shared/api/http/fetcher.ts": "export const fetcher = 1;\n",
  "src/shared/api/operation/stubbed.ts":
    'import { fetcher } from "../http/fetcher";\nexport const s = fetcher;\n',
  "src/shared/api/generated/admin.gen.ts": "export const g = 1;\n",
  "src/shared/api/stubs/data/status.ts":
    'import { g } from "../../generated/admin.gen";\nexport const x = g;\n',
  "src/shared/api/stubs/data/leaky.ts":
    'import { v } from "../../../../app/entry";\nexport const y = v;\n',
  "src/shared/api/stubs/data/uses-lib.ts":
    'import { l } from "../../../lib/core";\nexport const z = l;\n',
  "src/shared/lib/core/index.ts": "export const l = 1;\n",
  "src/app/entry.ts": "export const v = 1;\n",
  "src/pages/status/ui/Peeks.ts":
    'import { x } from "../../../shared/api/stubs/data/status";\nexport const p = x;\n',
  "src/pages/status/ui/Transport.ts":
    'import { fetcher } from "../../../shared/api/http/fetcher";\nexport const t = fetcher;\n',
  "src/pages/status/ui/Generated.ts":
    'import { g } from "../../../shared/api/generated/admin.gen";\nexport const q = g;\n',
  "src/pages/status/ui/Public.ts":
    'import { api } from "../../../shared/api";\nexport const ok = api;\n',
  "src/widgets/shell/ui/Operation.ts":
    'import { s } from "../../../shared/api/operation/stubbed";\nexport const w = s;\n',
};

// Пакет ESM-only и лежит в зависимостях web/, а не корня: подключаем по полю main его package.json.
async function loadCruiser(): Promise<Cruiser> {
  const dir = join(web, "node_modules", "dependency-cruiser");
  const manifest = JSON.parse(readFileSync(join(dir, "package.json"), "utf8")) as { main: string };
  return (await import(pathToFileURL(join(dir, manifest.main)).href)) as Cruiser;
}

async function cruiseFixture(root: string): Promise<Violation[]> {
  for (const [path, content] of Object.entries(FIXTURE)) {
    mkdirSync(dirname(join(root, path)), { recursive: true });
    writeFileSync(join(root, path), content);
  }
  const config = (await import(pathToFileURL(join(web, ".dependency-cruiser.ts")).href)) as {
    default: { forbidden: unknown[] };
  };
  const { cruise } = await loadCruiser();
  // Пути модулей в отчёте считаются от cwd, а правила написаны от корня web/ (`^src/…`): cwd на время прогона — фикстура.
  const previous = process.cwd();
  process.chdir(root);
  try {
    const result = await cruise(["src"], {
      validate: true,
      tsPreCompilationDeps: true,
      ruleSet: { forbidden: config.default.forbidden },
    });
    return toViolations(result);
  } finally {
    process.chdir(previous);
  }
}

type CruiseResult = Awaited<ReturnType<Cruiser["cruise"]>>;

function toViolations(result: CruiseResult): Violation[] {
  return result.output.summary.violations.map((v) => ({
    rule: v.rule.name,
    from: v.from,
    to: v.to,
  }));
}

const root = mkdtempSync(join(tmpdir(), "crane-depcruise-"));
const violations = await cruiseFixture(root);
after(() => rmSync(root, { recursive: true, force: true }));

const about = (rule: string, from: string): Violation[] =>
  violations.filter((v) => v.rule === rule && v.from === from);

it("страница не импортирует заглушки напрямую", () => {
  // Arrange
  const from = "src/pages/status/ui/Peeks.ts";

  // Act
  const found = about(INTERNALS, from);

  // Assert
  assert.equal(found.length, 1);
  assert.equal(found[0]?.to, "src/shared/api/stubs/data/status.ts");
});

it("страница не импортирует транспорт напрямую", () => {
  // Arrange
  const from = "src/pages/status/ui/Transport.ts";

  // Act
  const found = about(INTERNALS, from);

  // Assert
  assert.equal(found.length, 1);
});

it("страница не импортирует сгенерированный код напрямую", () => {
  // Arrange
  const from = "src/pages/status/ui/Generated.ts";

  // Act
  const found = about(INTERNALS, from);

  // Assert
  assert.equal(found.length, 1);
});

it("виджет не импортирует мутаторы операций", () => {
  // Arrange
  const from = "src/widgets/shell/ui/Operation.ts";

  // Act
  const found = about(INTERNALS, from);

  // Assert
  assert.equal(found.length, 1);
});

it("публичный src/shared/api/index.ts доступен всем", () => {
  // Arrange
  const from = "src/pages/status/ui/Public.ts";

  // Act
  const found = [...about(INTERNALS, from), ...about(STUBS_ONLY, from)];

  // Assert
  assert.deepEqual(found, []);
});

it("внутри shared/api модули свободно видят друг друга", () => {
  // Arrange
  const from = "src/shared/api/operation/stubbed.ts";

  // Act
  const found = about(INTERNALS, from);

  // Assert
  assert.deepEqual(found, []);
});

it("модуль заглушки не импортирует app", () => {
  // Arrange
  const from = "src/shared/api/stubs/data/leaky.ts";

  // Act
  const found = about(STUBS_ONLY, from);

  // Assert
  assert.equal(found.length, 1);
});

it("модуль заглушки видит shared/api и shared/lib", () => {
  // Arrange
  const sources = ["src/shared/api/stubs/data/status.ts", "src/shared/api/stubs/data/uses-lib.ts"];

  // Act
  const found = violations.filter((v) => v.rule === STUBS_ONLY && sources.includes(v.from));

  // Assert
  assert.deepEqual(found, []);
});
