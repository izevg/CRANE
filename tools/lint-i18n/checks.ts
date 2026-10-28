import { readdirSync, readFileSync } from "node:fs";
import { join, relative, sep } from "node:path";
import ts from "typescript";

const CYRILLIC = /[А-Яа-яЁё]/;
const SOURCE = /\.tsx?$/;
const SKIPPED_FILE = /\.(test|stories)\.tsx?$/;
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
  const found: ts.Node[] = [];
  const visit = (node: ts.Node): void => {
    if (
      isLiteralText(node) &&
      CYRILLIC.test(node.getText(sourceFile)) &&
      !isInsideMacro(node, source)
    ) {
      found.push(node);
    }
    ts.forEachChild(node, visit);
  };
  visit(sourceFile);
  return found;
}

/** Нарушения в одном исходнике: кириллический литерал вне макросов и исключений. */
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

/** Проверка одного файла: [] — чисто; иначе список сообщений. */
export function checkSource(source: string, fileName: string): string[] {
  return findViolations(source, fileName).map(
    (item) =>
      `${fileName}:${item.line} кириллический литерал вне t/msg/plural/Trans: ${item.text} — оберните в макрос Lingui или, если это контент витрины, поставьте над объявлением «// i18n-fixture: причина»`,
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

/**
 * Проверяемые файлы: `web/src` без тестов и stories и `web/screens` целиком (там stories — это и есть экраны, их
 * тексты тоже должны быть переведены, а файлы данных проверяются как остальные). Контент витрины (имена пакетов,
 * ФИО, описания, события журнала) остаётся русским и помечается над объявлением строкой `// i18n-fixture: причина`.
 */
export function checkTree(root: string): string[] {
  const web = join(root, "web");
  const files = [
    ...walk(join(web, "src")).filter((f) => SOURCE.test(f) && !SKIPPED_FILE.test(f)),
    ...walk(join(web, "screens")).filter((f) => SOURCE.test(f)),
  ];
  return files.flatMap((file) =>
    checkSource(readFileSync(file, "utf8"), relative(root, file).split(sep).join("/")),
  );
}
