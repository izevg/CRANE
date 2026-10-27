import { existsSync, readdirSync, readFileSync } from "node:fs";
import { join, relative, sep } from "node:path";
import { matcher, type Rules } from "./checks.ts";

type Lang = "rust" | "ts";

export interface TestFn {
  path: string;
  /** Строка объявления (1-based): к ней привязано нарушение. */
  line: number;
  name: string;
  lang: Lang;
  /** Строки между открывающей скобкой сигнатуры и закрывающей строкой на отступе объявления. */
  body: string[];
}

export interface Found {
  tests: TestFn[];
  violations: string[];
}

const MARKERS = ["// Arrange", "// Act", "// Assert"] as const;
const [ARRANGE, ACT, ASSERT] = MARKERS;

// Тест-атрибут: #[test], #[tokio::test], #[tokio::test(flavor = "...")]; не #[cfg(test)].
const RUST_TEST_ATTR = /^\s*#\[(\w+::)*test(\(.*\))?\]\s*$/;
// Всё, что похоже на тест-атрибут: если не разобрали строго, это нарушение, а не тихий пропуск.
const RUST_TEST_ATTR_LOOSE = /^\s*#\[(\w+::)*test\b/;
const RUST_FN = /^(\s*)(pub(\(\w+\))? )?(async )?fn (\w+)/;
const RUST_SCENARIO_CALL = /^\s+(\w+)\(&new\(\)\)\.await;$/;
const TS_TEST = /^(\s*)(it|test)(\.\w+)*\(/;
const TS_TITLE = /(["'`])(.+?)\1/;
const BINDING_CALL = /\b\w+_contract\(/;
const ASSERT_BEFORE: Record<Lang, RegExp> = {
  rust: /\bassert(_eq|_ne)?!\(/,
  ts: /^\s*(assert\.|assert\(|expect\()/,
};
const LINE_BREAK = /\r?\n/;
const SHOULD_PANIC = /\bshould_panic\b/;
const LEADING_WS = /^\s*/;
// Не оператор: пробел (продолжение глубже), комментарий, закрывающие скобки и цепочки `.`/`?`.
const CONTINUATION = /^(\s|\/\/|[)\]}.?])/;
const TS_FILE = /\.tsx?$/;
const UNKNOWN = "<тест>";
const PLURAL_FEW_MAX = 4;

const report = (path: string, line: number, name: string, rule: string, todo: string): string =>
  `${path}:${line} ${name} — ${rule} — ${todo}`;

const splitLines = (src: string): string[] => src.split(LINE_BREAK);
const at = (lines: string[], i: number): string => lines[i] ?? "";
const isAttr = (line: string): boolean => line.trim().startsWith("#[");
const isLineComment = (line: string): boolean => line.trim().startsWith("//");

const unrecognized = (path: string, line: number, name: string, why: string): string =>
  report(
    path,
    line,
    name,
    `тест не распознан (${why})`,
    "приведите тест к форме rustfmt/Biome: обычная fn или it(...) с телом в фигурных скобках",
  );

/** Тело — строки после сигнатуры до первой строки ровно `<indent><closing>`; нет границ — undefined. */
function extractBody(
  lines: string[],
  start: number,
  indent: string,
  closing: string,
): string[] | undefined {
  const head = at(lines, start).trimEnd();
  if (head.endsWith("}") || head.endsWith(";")) {
    return undefined;
  }
  const end = lines.findIndex((l, i) => i > start && l === `${indent}${closing}`);
  if (end < 0) {
    return undefined;
  }
  const open = lines.findIndex((l, i) => i >= start && i < end && l.trimEnd().endsWith("{"));
  return open < 0 ? undefined : lines.slice(open + 1, end);
}

const unclosed = (path: string, line: number, name: string): string =>
  report(
    path,
    line,
    name,
    "не найдено тело теста",
    "закройте тело на отступе объявления (`}` / `});`) и откройте `{` в конце строки сигнатуры",
  );

function rustTestAt(
  lines: string[],
  attrLine: number,
  path: string,
): { found: Found; next: number } {
  const found: Found = { tests: [], violations: [] };
  let from = attrLine;
  while (from > 0 && isAttr(at(lines, from - 1))) {
    from -= 1;
  }
  let declIdx = attrLine + 1;
  while (
    declIdx < lines.length &&
    (isAttr(at(lines, declIdx)) || isLineComment(at(lines, declIdx)))
  ) {
    declIdx += 1;
  }
  const next = declIdx - 1;
  const decl = RUST_FN.exec(at(lines, declIdx));
  if (!decl) {
    found.violations.push(
      unrecognized(path, attrLine + 1, UNKNOWN, "после атрибутов нет `fn имя`"),
    );
    return { found, next };
  }
  const [, indent = "", , , , name = UNKNOWN] = decl;
  const line = declIdx + 1;
  if (lines.slice(from, declIdx).some((l) => SHOULD_PANIC.test(l))) {
    found.violations.push(
      report(
        path,
        line,
        name,
        "#[should_panic] запрещён",
        "проверяйте ошибку значением (Result/Option) в Assert",
      ),
    );
  }
  const body = extractBody(lines, declIdx, indent, "}");
  if (body) {
    found.tests.push({ path, line, name, lang: "rust", body });
  } else {
    found.violations.push(unclosed(path, line, name));
  }
  return { found, next };
}

/** Тесты `#[test]`/`#[tokio::test]` в исходнике Rust. */
export function findRustTests(src: string, path: string): Found {
  const lines = splitLines(src);
  const result: Found = { tests: [], violations: [] };
  for (let i = 0; i < lines.length; i += 1) {
    const line = at(lines, i);
    if (RUST_TEST_ATTR.test(line)) {
      const { found, next } = rustTestAt(lines, i, path);
      result.tests.push(...found.tests);
      result.violations.push(...found.violations);
      i = next;
    } else if (RUST_TEST_ATTR_LOOSE.test(line)) {
      result.violations.push(
        unrecognized(path, i + 1, UNKNOWN, "атрибут теста в нестандартной форме"),
      );
    }
  }
  return result;
}

/** Сценарии контракта: имена из вызовов `name(&new()).await;` — тела `async fn name` в том же файле. */
export function findContractScenarios(src: string, path: string): Found {
  const lines = splitLines(src);
  const result: Found = { tests: [], violations: [] };
  const seen = new Set<string>();
  lines.forEach((text, i) => {
    const name = RUST_SCENARIO_CALL.exec(text)?.[1];
    if (name === undefined || seen.has(name)) {
      return;
    }
    seen.add(name);
    const declRe = new RegExp(`^(\\s*)(pub )?async fn ${name}\\b`);
    const declIdx = lines.findIndex((l) => declRe.test(l));
    if (declIdx < 0) {
      result.violations.push(
        report(
          path,
          i + 1,
          name,
          "сценарий контракта без функции",
          "объявите `async fn` сценария в этом же файле",
        ),
      );
      return;
    }
    const indent = declRe.exec(at(lines, declIdx))?.[1] ?? "";
    const body = extractBody(lines, declIdx, indent, "}");
    if (body) {
      result.tests.push({ path, line: declIdx + 1, name, lang: "rust", body });
    } else {
      result.violations.push(unclosed(path, declIdx + 1, name));
    }
  });
  return result;
}

/** Тесты `it(...)`/`test(...)` (в т.ч. `.only`, `.skip`, `.each(...)(...)`) в исходнике TS. */
export function findTsTests(src: string, path: string): Found {
  const lines = splitLines(src);
  const result: Found = { tests: [], violations: [] };
  lines.forEach((text, i) => {
    const indent = TS_TEST.exec(text)?.[1];
    if (indent === undefined) {
      return;
    }
    const name = TS_TITLE.exec(`${text}\n${at(lines, i + 1)}`)?.[2] ?? UNKNOWN;
    const body = extractBody(lines, i, indent, "});");
    if (body) {
      result.tests.push({ path, line: i + 1, name, lang: "ts", body });
    } else {
      result.violations.push(unclosed(path, i + 1, name));
    }
  });
  return result;
}

const indentOf = (line: string): string => LEADING_WS.exec(line)?.[0] ?? "";

/** Оператор начинается на отступе тела; продолжения глубже или начинаются с `) ] } . ?`. */
const startsStatement = (line: string, indent: string): boolean => {
  if (!line.startsWith(indent)) {
    return false;
  }
  const rest = line.slice(indent.length);
  return rest !== "" && !CONTINUATION.test(rest);
};

const countStatements = (lines: string[], indent: string): number =>
  lines.filter((l) => startsStatement(l, indent)).length;

// Числа 5+ в Act редкость: достаточно различать «2–4 оператора» и «операторов».
const plural = (n: number): string => (n <= PLURAL_FEW_MAX ? "оператора" : "операторов");

function markerViolations(t: TestFn, fail: (rule: string, todo: string) => string): string[] {
  const positions = MARKERS.map((m) => t.body.flatMap((l, i) => (l.trim() === m ? [i] : [])));
  const wrong = MARKERS.flatMap((m, k) =>
    positions[k]?.length === 1 ? [] : [`${m} ×${positions[k]?.length ?? 0}`],
  );
  if (wrong.length > 0) {
    return [
      fail(
        `маркеры: ${wrong.join(", ")}`,
        `каждый из ${MARKERS.join(", ")} должен стоять в теле ровно один раз отдельной строкой`,
      ),
    ];
  }
  const [a = -1, b = -1, c = -1] = positions.map((x) => x[0] ?? -1);
  return a < b && b < c
    ? []
    : [
        fail(
          "маркеры не по порядку",
          `расставьте ${ARRANGE}, ${ACT}, ${ASSERT} именно в этом порядке`,
        ),
      ];
}

/** Нарушения одного теста; пусто — тест в форме AAA. */
export function checkAaa(t: TestFn): string[] {
  const fail = (rule: string, todo: string): string => report(t.path, t.line, t.name, rule, todo);
  const indent = indentOf(t.body.find((l) => l.trim() !== "") ?? "");
  const text = t.body.join("\n");
  // Привязка не проверяет: оператор с assert под исключение не подпадает.
  const isBinding =
    countStatements(t.body, indent) === 1 &&
    BINDING_CALL.test(text) &&
    !ASSERT_BEFORE[t.lang].test(text);
  if (isBinding) {
    return [];
  }
  const found = markerViolations(t, fail);
  if (found.length > 0) {
    return found;
  }
  const idx = (m: string): number => t.body.findIndex((l) => l.trim() === m);
  const [act, assertAt] = [idx(ACT), idx(ASSERT)];
  const actStatements = countStatements(t.body.slice(act + 1, assertAt), indent);
  if (actStatements !== 1) {
    found.push(
      actStatements === 0
        ? fail("в Act нет оператора", "поместите в Act вызов проверяемого поведения")
        : fail(
            `в Act ${actStatements} ${plural(actStatements)}`,
            "оставьте в Act одно действие, подготовку перенесите в Arrange",
          ),
    );
  }
  const early = t.body
    .slice(0, assertAt)
    .find((l) => !isLineComment(l) && ASSERT_BEFORE[t.lang].test(l));
  if (early !== undefined) {
    found.push(fail("проверка до // Assert", `перенесите «${early.trim()}» в секцию Assert`));
  }
  return found;
}

const violationsOfFound = ({ tests, violations }: Found): string[] => [
  ...violations,
  ...tests.flatMap(checkAaa),
];

/** Нарушения AAA в одном исходнике; тип по расширению, прочие файлы пропускаются. */
export function checkAaaSource(src: string, path: string): string[] {
  if (path.endsWith(".rs")) {
    return [
      ...violationsOfFound(findRustTests(src, path)),
      ...violationsOfFound(findContractScenarios(src, path)),
    ];
  }
  return violationsOfFound(findTsTests(src, path));
}

/** AAA во всех тестах под rules.aaa.roots (e2e и ignoredDirs пропускаются). */
export function checkAaaTree(root: string, rules: Rules): string[] {
  const isTest = matcher(rules.testFiles);
  const skipped = new Set([...rules.ignoredDirs, ...rules.aaa.exemptDirs]);
  const isTarget = (name: string): boolean =>
    name.endsWith(".rs") || (TS_FILE.test(name) && isTest(name));
  const walk = (dir: string): string[] =>
    readdirSync(dir, { withFileTypes: true })
      .sort((a, b) => a.name.localeCompare(b.name))
      .flatMap((entry) => {
        const full = join(dir, entry.name);
        if (entry.isDirectory()) {
          return skipped.has(entry.name) || entry.name.startsWith(".") ? [] : walk(full);
        }
        if (!isTarget(entry.name)) {
          return [];
        }
        const rel = relative(root, full).split(sep).join("/");
        return checkAaaSource(readFileSync(full, "utf8"), rel);
      });
  return rules.aaa.roots
    .map((r) => join(root, r))
    .filter((start) => existsSync(start))
    .flatMap(walk);
}
