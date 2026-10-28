import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test as it } from "node:test";
import {
  type CargoMetadata,
  checkRustLayers,
  checkSerde,
  checkStructure,
  type Rules,
} from "./checks.ts";

const MAX_DIRS = 8;

const rules: Rules = {
  scope: ["src"],
  limits: { files: 4, dirs: MAX_DIRS },
  ignoredDirs: ["node_modules"],
  unlimitedDirs: ["tests"],
  serviceFiles: ["Cargo.toml"],
  testFiles: ["*.test.*"],
  storyFiles: ["*.stories.*"],
  entryFiles: ["lib.rs", "index.ts"],
  rustLayers: {
    domain: [],
    ports: ["domain"],
    wire: ["domain"],
    formats: ["domain", "ports"],
    adapters: ["domain", "ports", "wire"],
    entry: ["domain", "ports", "wire", "adapters"],
  },
  serde: { crates: ["serde", "serde_*", "serde-*"], layers: ["formats", "entry", "wire"] },
  aaa: { roots: [], exemptDirs: [] },
  size: { roots: [], exemptDirs: [], extensions: [], maxLines: 400, warnLines: 320 },
};

const tree = (spec: Record<string, 0 | 1>): string => {
  const root = mkdtempSync(join(tmpdir(), "ls-"));
  for (const [path, isDir] of Object.entries(spec)) {
    if (isDir) {
      mkdirSync(join(root, path), { recursive: true });
    } else {
      mkdirSync(join(root, path, ".."), { recursive: true });
      writeFileSync(join(root, path), "");
    }
  }
  return root;
};

const dirs = (n: number): Record<string, 0 | 1> =>
  Object.fromEntries(Array.from({ length: n }, (_, i) => [`src/d${i}`, 1]));
const files = (names: string[]): Record<string, 0 | 1> =>
  Object.fromEntries(names.map((n) => [`src/${n}`, 0]));

interface StructureCase {
  name: string;
  spec: Record<string, 0 | 1>;
  expected: number;
}

const structureCases: StructureCase[] = [
  { name: "4 файла в папке допустимы", spec: files(["a.ts", "b.ts", "c.ts", "d.ts"]), expected: 0 },
  {
    name: "5 файлов в папке — нарушение",
    spec: files(["a.ts", "b.ts", "c.ts", "d.ts", "e.ts"]),
    expected: 1,
  },
  { name: "8 подпапок допустимы", spec: dirs(MAX_DIRS), expected: 0 },
  { name: "9 подпапок — нарушение", spec: dirs(MAX_DIRS + 1), expected: 1 },
  { name: "смешанное наполнение запрещено", spec: { "src/a.ts": 0, "src/d": 1 }, expected: 1 },
  {
    name: "файл-вход не создаёт смешанность",
    spec: { "src/index.ts": 0, "src/d": 1 },
    expected: 0,
  },
  {
    name: "файл теста не создаёт смешанность",
    spec: { "src/a.test.ts": 0, "src/d": 1 },
    expected: 0,
  },
  {
    name: "stories не считаются в лимит файлов и не создают смешанность",
    spec: {
      ...files(["a.ts", "b.ts", "c.ts", "d.ts"]),
      "src/a.stories.tsx": 0,
      "src/b.stories.tsx": 0,
    },
    expected: 0,
  },
  {
    name: "файл story не создаёт смешанность",
    spec: { "src/a.stories.tsx": 0, "src/d": 1 },
    expected: 0,
  },
  {
    name: "служебные файлы и tests/ не считаются",
    spec: { ...files(["a.ts", "b.ts", "c.ts", "d.ts"]), "src/Cargo.toml": 0, "src/tests/x.ts": 0 },
    expected: 0,
  },
];

for (const c of structureCases) {
  it(`правило четырёх: ${c.name}`, () => {
    // Arrange
    const root = tree(c.spec);

    // Act
    const actual = checkStructure(root, rules).length;

    // Assert
    assert.equal(actual, c.expected);
  });
}

const meta = (root: string, pkgs: [string, string, string[]][]): CargoMetadata => ({
  packages: pkgs.map(([name, path, deps]) => ({
    name,
    version: "0.0.0",
    manifest_path: join(root, "crates", path, "Cargo.toml"),
    dependencies: deps.map((d) => ({ name: d, kind: null })),
  })),
});

const layerCases = [
  {
    name: "domain не может зависеть от ports",
    pkgs: [
      ["d", "core/domain", ["p"]],
      ["p", "core/ports", []],
    ] as [string, string, string[]][],
    expected: 1,
  },
  {
    name: "adapters может зависеть от ports и domain",
    pkgs: [
      ["d", "core/domain", []],
      ["p", "core/ports", ["d"]],
      ["pg", "adapters/persistence/pg", ["p", "d"]],
    ] as [string, string, string[]][],
    expected: 0,
  },
  {
    name: "wire может зависеть от domain",
    pkgs: [
      ["d", "core/domain", []],
      ["w", "wire/feeds", ["d"]],
    ] as [string, string, string[]][],
    expected: 0,
  },
  {
    name: "wire не может зависеть от ports",
    pkgs: [
      ["p", "core/ports", []],
      ["w", "wire/feeds", ["p"]],
    ] as [string, string, string[]][],
    expected: 1,
  },
  {
    name: "adapters может зависеть от wire",
    pkgs: [
      ["w", "wire/feeds", []],
      ["pg", "adapters/persistence/pg", ["w"]],
    ] as [string, string, string[]][],
    expected: 0,
  },
  {
    name: "formats не может зависеть от wire",
    pkgs: [
      ["w", "wire/feeds", []],
      ["f", "formats/npm", ["w"]],
    ] as [string, string, string[]][],
    expected: 1,
  },
];

for (const c of layerCases) {
  it(`слои крейтов: ${c.name}`, () => {
    // Arrange
    const root = "/r";
    const metadata = meta(root, c.pkgs);

    // Act
    const actual = checkRustLayers(root, rules, metadata).length;

    // Assert
    assert.equal(actual, c.expected);
  });
}

const serdeCases = [
  {
    name: "adapters → serde",
    path: "adapters/persistence/pg",
    dep: "serde",
    kind: null,
    expected: 1,
  },
  {
    name: "adapters → serde_json",
    path: "adapters/persistence/pg",
    dep: "serde_json",
    kind: null,
    expected: 1,
  },
  {
    name: "adapters → serde-xml (build-зависимость)",
    path: "adapters/persistence/pg",
    dep: "serde-xml",
    kind: "build",
    expected: 1,
  },
  { name: "entry → serde", path: "entry/http", dep: "serde", kind: null, expected: 0 },
  { name: "formats → serde", path: "formats/npm", dep: "serde", kind: null, expected: 0 },
  { name: "wire → serde", path: "wire/feeds", dep: "serde", kind: null, expected: 0 },
  {
    name: "domain → serde как dev-зависимость",
    path: "core/domain",
    dep: "serde",
    kind: "dev",
    expected: 0,
  },
  {
    name: "app → serdex (не совпадает с шаблоном)",
    path: "core/app",
    dep: "serdex",
    kind: null,
    expected: 0,
  },
];

for (const c of serdeCases) {
  it(`serde: ${c.name}`, () => {
    // Arrange
    const root = "/r";
    const metadata: CargoMetadata = {
      packages: [
        {
          name: "crate-under-test",
          version: "0.0.0",
          manifest_path: join(root, "crates", c.path, "Cargo.toml"),
          dependencies: [{ name: c.dep, kind: c.kind }],
        },
      ],
    };

    // Act
    const actual = checkSerde(root, rules, metadata).length;

    // Assert
    assert.equal(actual, c.expected);
  });
}
