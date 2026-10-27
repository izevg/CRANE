import { readdirSync, readFileSync } from "node:fs";
import { join, relative, sep } from "node:path";
import ts from "typescript";

/** Файл, ещё не переведённый на i18n: строка в первых строках файла. Поток миграции удаляет её вместе с литералами. */
export const MARKER = "// i18n: не мигрирован";

const CYRILLIC = /[А-Яа-яЁё]/;
const MARKER_SCAN_LINES = 5;
const SOURCE = /\.tsx?$/;
const SKIPPED_FILE = /\.(test|stories)\.tsx?$/;
/**
 * Фикстуры витрины: пользовательский контент (имена пакетов, описания, ФИО, события журнала), а не текст интерфейса.
 * Они остаются русскими, переводить их незачем; тексты интерфейса экранов лежат в самих stories.
 */
const FIXTURE_DIR = "web/screens/data/";
const IGNORE_COMMENT = /\/\/ i18n-ignore: \S/;
/** Комментарий над объявлением: всё объявление — фикстура (контент витрины), а не текст интерфейса. */
const FIXTURE_COMMENT = /^\/\/ i18n-fixture: \S/;
const STORY_TYPE = /^Story\b/;
const META_TYPE = /^Meta</;
const STORY_META_KEYS = new Set(["title", "name"]);

/** Макросы Lingui: текст внутри них — сообщение каталога, а не литерал. */
const MACRO_TAGS = new Set(["t", "msg", "defineMessage"]);
const MACRO_CALLS = new Set(["t", "msg", "defineMessage", "plural", "select", "selectOrdinal"]);

type Found = { line: number; text: string };

function calleeName(node: ts.CallExpression | ts.TaggedTemplateExpression): string | undefined {
  const expression = ts.isCallExpression(node) ? node.expression : node.tag;
  return ts.isIdentifier(expression) ? expression.text : undefined;
}

function isTransElement(node: ts.Node): boolean {
  return (
    (ts.isJsxElement(node) && node.openingElement.tagName.getText() === "Trans") ||
    (ts.isJsxSelfClosingElement(node) && node.tagName.getText() === "Trans")
  );
}

/** Узел, внутри которого текст не считается литералом: макрос Lingui, `<Trans>`, `throw new Error`, импорт. */
function isExemptContainer(node: ts.Node): boolean {
  if (ts.isTaggedTemplateExpression(node) || ts.isCallExpression(node)) {
    const names = ts.isCallExpression(node) ? MACRO_CALLS : MACRO_TAGS;
    return names.has(calleeName(node) ?? "");
  }
  if (ts.isNewExpression(node)) {
    // сообщение об ошибке для разработчика не показывается пользователю
    return ts.isThrowStatement(node.parent);
  }
  return ts.isImportDeclaration(node) || isTransElement(node);
}

/** Объект истории или меты Storybook: `const meta = {…} satisfies Meta<…>` и `const X: Story = {…}`. */
function isStoryObject(node: ts.Node): boolean {
  if (!ts.isObjectLiteralExpression(node)) {
    return false;
  }
  const parent = node.parent;
  if (ts.isSatisfiesExpression(parent)) {
    return META_TYPE.test(parent.type.getText());
  }
  return (
    ts.isVariableDeclaration(parent) &&
    parent.type !== undefined &&
    STORY_TYPE.test(parent.type.getText())
  );
}

/**
 * Служебные части stories не являются текстом интерфейса: тело `play` (тест, ищет подписи), `title` и `name`
 * самой истории или меты. Текст интерфейса экрана внутри них всё равно оборачивается в макрос.
 */
function isStoryService(node: ts.Node): boolean {
  if (!(ts.isPropertyAssignment(node) && ts.isIdentifier(node.name))) {
    return false;
  }
  if (node.name.text === "play") {
    return true;
  }
  return STORY_META_KEYS.has(node.name.text) && isStoryObject(node.parent);
}

function isFixtureStatement(node: ts.Node, source: string): boolean {
  if (!ts.isStatement(node)) {
    return false;
  }
  const ranges = ts.getLeadingCommentRanges(source, node.getFullStart()) ?? [];
  return ranges.some((range) => FIXTURE_COMMENT.test(source.slice(range.pos, range.end)));
}

function isInsideMacro(node: ts.Node, source: string): boolean {
  let current: ts.Node | undefined = node.parent;
  while (current !== undefined) {
    if (
      isExemptContainer(current) ||
      isStoryService(current) ||
      isFixtureStatement(current, source)
    ) {
      return true;
    }
    current = current.parent;
  }
  return false;
}

function isLiteralText(node: ts.Node): boolean {
  return (
    ts.isStringLiteral(node) ||
    ts.isNoSubstitutionTemplateLiteral(node) ||
    ts.isTemplateHead(node) ||
    ts.isTemplateMiddle(node) ||
    ts.isTemplateTail(node) ||
    ts.isJsxText(node)
  );
}

/** Узлы-литералы с кириллицей вне макросов и исключений: то, что нужно перевести. */
export function violationNodes(sourceFile: ts.SourceFile): ts.Node[] {
  const source = sourceFile.text;
  const lines = source.split("\n");
  const found: ts.Node[] = [];
  const visit = (node: ts.Node): void => {
    if (
      isLiteralText(node) &&
      CYRILLIC.test(node.getText(sourceFile)) &&
      !isInsideMacro(node, source)
    ) {
      const line = sourceFile.getLineAndCharacterOfPosition(node.getStart(sourceFile)).line;
      const near = `${lines[line - 1] ?? ""}\n${lines[line] ?? ""}`;
      if (!IGNORE_COMMENT.test(near)) {
        found.push(node);
      }
    }
    ts.forEachChild(node, visit);
  };
  visit(sourceFile);
  return found;
}

/** Нарушения в одном исходнике: кириллический литерал вне макросов. Пусто, если файл помечен маркером. */
export function findViolations(source: string, fileName: string): Found[] {
  const sourceFile = ts.createSourceFile(
    fileName,
    source,
    ts.ScriptTarget.Latest,
    true,
    fileName.endsWith("x") ? ts.ScriptKind.TSX : ts.ScriptKind.TS,
  );
  return violationNodes(sourceFile).map((node) => ({
    line: sourceFile.getLineAndCharacterOfPosition(node.getStart(sourceFile)).line + 1,
    text: node.getText(sourceFile).trim().slice(0, 60),
  }));
}

export function hasMarker(source: string): boolean {
  return source
    .split("\n")
    .slice(0, MARKER_SCAN_LINES)
    .some((line) => line.trim() === MARKER);
}

/** Проверка одного файла: [] — чисто; иначе список сообщений. */
export function checkSource(source: string, fileName: string): string[] {
  const found = findViolations(source, fileName);
  if (hasMarker(source)) {
    return found.length === 0
      ? [`${fileName}: маркер «${MARKER}» больше не нужен — литералов нет, удалите строку`]
      : [];
  }
  return found.map(
    (item) =>
      `${fileName}:${item.line} кириллический литерал вне t/msg/plural/Trans: ${item.text} — оберните в макрос Lingui или поставьте маркер «${MARKER}»`,
  );
}

function walk(dir: string): string[] {
  return readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    if (entry.name === "node_modules" || entry.name.startsWith(".")) {
      return [];
    }
    const full = join(dir, entry.name);
    return entry.isDirectory() ? walk(full) : [full];
  });
}

function isFixture(path: string): boolean {
  return path.split(sep).join("/").startsWith(FIXTURE_DIR);
}

/**
 * Проверяемые файлы: `web/src` без тестов и stories (фикстуры витрины на русском) и `web/screens` целиком
 * (там stories — это и есть экраны, их тексты тоже должны быть переведены).
 */
export function checkTree(root: string): string[] {
  const web = join(root, "web");
  const files = [
    ...walk(join(web, "src")).filter((f) => SOURCE.test(f) && !SKIPPED_FILE.test(f)),
    ...walk(join(web, "screens")).filter((f) => SOURCE.test(f)),
  ].filter((file) => !isFixture(relative(root, file)));
  return files.flatMap((file) =>
    checkSource(readFileSync(file, "utf8"), relative(root, file).split(sep).join("/")),
  );
}
