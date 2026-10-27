import { readdirSync, readFileSync } from "node:fs";
import { join, relative, sep } from "node:path";
import ts from "typescript";

/** Исключение из правил: комментарий с причиной в той же или предыдущей строке. */
const IGNORE_COMMENT = /\/\/ motion-ignore: \S|\/\* motion-ignore: \S/;

const TS_SOURCE = /\.tsx?$/;
const CSS_SOURCE = /\.css$/;
const SKIPPED_FILE = /\.test\.tsx?$/;

/** Свойства, анимация которых запускает Layout: spec 2 допускает только transform и opacity (цвет — для hover). */
const LAYOUT_PROPS =
  /\b(width|height|top|left|right|bottom|margin|padding|inset|size|gap|flex|grid)\b/;
const BRACKET_PROPS = /^transition-\[(.*)\]$/;
const NUMERIC_DURATION = /^duration-(\d+|\[[\d.]+m?s\])$/;
const CSS_ALL = /\btransition(-property)?\s*:\s*all\b/;
// Шорткат `transition: 200ms ...` без имени свойства означает `all`
const CSS_SHORTHAND_WITHOUT_PROPERTY = /\btransition\s*:\s*[\d.]+m?s\b/;
const CSS_NUMERIC_DURATION = /\btransition(-duration)?\s*:[^;]*\b(?!0m?s\b)\d+(\.\d+)?m?s\b/;
const CSS_COMMENT = /\/\*.*?\*\//g;
const WHITESPACE = /\s+/;
const IMPORTANT = /^!/;
const UNDERSCORES = /_/g;

type Found = { line: number; message: string };

/** Класс без вариантов (`hover:`, `md:`, `group-data-[x]/y:`): всё после последнего `:` вне квадратных скобок. */
function utilityOf(token: string): string {
  let depth = 0;
  let cut = -1;
  for (let i = 0; i < token.length; i += 1) {
    const ch = token[i];
    if (ch === "[" || ch === "(") {
      depth += 1;
    } else if (ch === "]" || ch === ")") {
      depth -= 1;
    } else if (ch === ":" && depth === 0) {
      cut = i;
    }
  }
  return token.slice(cut + 1).replace(IMPORTANT, "");
}

/** Нарушение для одного класса Tailwind или `undefined`. */
export function checkClass(token: string): string | undefined {
  const utility = utilityOf(token);
  if (utility === "transition") {
    return "голый `transition` анимирует широкий список свойств: укажите `transition-colors`, `transition-opacity`, `transition-transform` или `transition-[…]`";
  }
  if (utility === "transition-all") {
    return "`transition-all` запрещён: перечислите свойства явно";
  }
  const bracket = BRACKET_PROPS.exec(utility);
  if (bracket?.[1] !== undefined && LAYOUT_PROPS.test(bracket[1].replace(UNDERSCORES, ","))) {
    return `\`${utility}\` анимирует свойство раскладки (Layout): допустимы transform, opacity, цвет`;
  }
  if (NUMERIC_DURATION.test(utility)) {
    return `\`${utility}\`: длительность берите из токенов, например \`duration-(--dur-fast)\``;
  }
  return undefined;
}

/** Нарушения в строке CSS или в тексте inline-стиля. */
export function checkCssText(text: string): string | undefined {
  if (CSS_ALL.test(text)) {
    return "`transition: all` запрещён: перечислите свойства явно";
  }
  if (CSS_SHORTHAND_WITHOUT_PROPERTY.test(text)) {
    return "`transition: <время>` без имени свойства означает `all`: укажите свойство";
  }
  return undefined;
}

function isIgnored(lines: readonly string[], line: number): boolean {
  return IGNORE_COMMENT.test(`${lines[line - 1] ?? ""}\n${lines[line] ?? ""}`);
}

function checkTs(source: string, fileName: string): Found[] {
  const lines = source.split("\n");
  const sourceFile = ts.createSourceFile(
    fileName,
    source,
    ts.ScriptTarget.Latest,
    true,
    fileName.endsWith("x") ? ts.ScriptKind.TSX : ts.ScriptKind.TS,
  );
  const found: Found[] = [];
  const add = (node: ts.Node, message: string) => {
    const line = sourceFile.getLineAndCharacterOfPosition(node.getStart(sourceFile)).line;
    if (!isIgnored(lines, line)) {
      found.push({ line: line + 1, message });
    }
  };
  const visit = (node: ts.Node): void => {
    if (
      ts.isStringLiteral(node) ||
      ts.isNoSubstitutionTemplateLiteral(node) ||
      ts.isTemplateHead(node) ||
      ts.isTemplateMiddle(node) ||
      ts.isTemplateTail(node)
    ) {
      for (const token of node.text.split(WHITESPACE)) {
        const message = checkClass(token);
        if (message !== undefined) {
          add(node, message);
        }
      }
      const css = checkCssText(node.text);
      if (css !== undefined) {
        add(node, css);
      }
    }
    ts.forEachChild(node, visit);
  };
  visit(sourceFile);
  return found;
}

function checkCss(source: string): Found[] {
  const lines = source.split("\n");
  const found: Found[] = [];
  lines.forEach((raw, index) => {
    const text = raw.replace(CSS_COMMENT, "");
    const message =
      checkCssText(text) ??
      (CSS_NUMERIC_DURATION.test(text) && !text.includes("var(")
        ? "длительность перехода берите из токенов `--dur-*`"
        : undefined);
    if (message !== undefined && !isIgnored(lines, index + 1)) {
      found.push({ line: index + 1, message });
    }
  });
  return found;
}

/** Проверка одного файла: [] — чисто. */
export function checkSource(source: string, fileName: string): string[] {
  const found = CSS_SOURCE.test(fileName) ? checkCss(source) : checkTs(source, fileName);
  return found.map((item) => `${fileName}:${item.line} ${item.message}`);
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

/** Проверяются `web/src` и `web/screens` без тестов. Токены движения (`motion.css`) — единственный источник чисел. */
export function checkTree(root: string): string[] {
  const web = join(root, "web");
  const files = [...walk(join(web, "src")), ...walk(join(web, "screens"))].filter(
    (file) =>
      (TS_SOURCE.test(file) || CSS_SOURCE.test(file)) &&
      !SKIPPED_FILE.test(file) &&
      !file.endsWith(`${sep}motion.css`),
  );
  return files.flatMap((file) =>
    checkSource(readFileSync(file, "utf8"), relative(root, file).split(sep).join("/")),
  );
}
