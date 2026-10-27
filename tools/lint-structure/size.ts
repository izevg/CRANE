import { readFileSync } from "node:fs";
import { join } from "node:path";
import { type Rules, sourceFiles } from "./checks.ts";

export interface Measured {
  path: string;
  lines: number;
}

const NEWLINE = /\n/g;

/** Физические строки как у `wc -l` (число `\n`), плюс одна, если последняя строка без перевода. */
export const lineCount = (text: string): number => {
  const breaks = text.match(NEWLINE)?.length ?? 0;
  return text.length > 0 && !text.endsWith("\n") ? breaks + 1 : breaks;
};

/** Размеры исходных файлов под rules.size.roots (generated, migrations и ignoredDirs пропускаются). */
export function measure(root: string, rules: Rules): Measured[] {
  const skipped = new Set([...rules.ignoredDirs, ...rules.size.exemptDirs]);
  return sourceFiles(root, rules.size.roots, skipped)
    .filter((path) => rules.size.extensions.some((ext) => path.endsWith(ext)))
    .map((path) => ({ path, lines: lineCount(readFileSync(join(root, path), "utf8")) }));
}

const byLinesDesc = (a: Measured, b: Measured): number =>
  b.lines - a.lines || a.path.localeCompare(b.path);

/** Нарушители лимита: самые большие первыми. */
export const oversized = (files: Measured[], max: number): string[] =>
  files
    .filter((f) => f.lines > max)
    .sort(byLinesDesc)
    .map((f) => `${f.path}: ${f.lines} строк при лимите ${max}`);

/** Отчёт без отказа: top самых больших файлов плюс всё, что дошло до порога предупреждения. */
export function sizeReport(files: Measured[], top: number, warn: number, max: number): string[] {
  const sorted = [...files].sort(byLinesDesc);
  const shown = sorted.filter((f, i) => i < top || f.lines >= warn);
  const mark = (lines: number): string => {
    if (lines > max) {
      return "нарушение";
    }
    return lines >= warn ? "предупреждение" : "";
  };
  const rows = shown.map((f) => `${f.lines}\t${f.path}\t${mark(f.lines)}`.trimEnd());
  const over = sorted.filter((f) => f.lines > max).length;
  const near = sorted.filter((f) => f.lines >= warn && f.lines <= max).length;
  return [
    `Файлов: ${files.length}; нарушений (> ${max}): ${over}; предупреждений (>= ${warn}): ${near}`,
    ...rows,
  ];
}
