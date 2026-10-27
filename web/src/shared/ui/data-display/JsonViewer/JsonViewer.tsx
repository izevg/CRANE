// i18n: не мигрирован
import { plural } from "@lingui/core/macro";
import { useLingui } from "@lingui/react/macro";
import {
  ChevronDownIcon,
  ChevronRightIcon,
  ChevronsDownUpIcon,
  ChevronsUpDownIcon,
} from "lucide-react";
import { type ReactNode, useCallback, useMemo, useState } from "react";
import { cx, type WithClassName } from "~/shared/lib/core";
import { CopyButton } from "../../actions";
import { Button } from "../../primitives";
import { collapseAll, defaultCollapsed, flatten, type JsonRow, type JsonValue } from "./tree";

export type { JsonValue } from "./tree";

const ROOT_NAME = "корень";
const COLLAPSE = "Свернуть";
const EXPAND = "Развернуть";
const EXPAND_ALL = "Развернуть всё";
const COLLAPSE_ALL = "Свернуть всё";
const COPY_JSON = "Копировать JSON";
const REGION = "JSON";
const INDENT_REM = 1;
const JSON_INDENT = 2;

type JsonViewerProps = WithClassName<{ value: JsonValue; label?: string }>;

function scalarClass(value: JsonValue | undefined): string {
  if (typeof value === "string") {
    return "text-(--code-string)";
  }
  return value === null ? "text-(--code-null)" : "text-(--code-number)";
}

function scalarText(value: JsonValue): string {
  return JSON.stringify(value);
}

function Punct({ children }: { children: ReactNode }) {
  return <span className="text-(--code-punct)">{children}</span>;
}

function brackets(row: JsonRow): { open: string; close: string } {
  return "container" in row && row.container === "array"
    ? { open: "[", close: "]" }
    : { open: "{", close: "}" };
}

/** «3 элемента» / «12 ключей»: свёрнутый контейнер. Склонение — ICU plural, порядок слов зависит от языка. */
function Summary({ row }: { row: Extract<JsonRow, { kind: "collapsed" }> }) {
  const { t } = useLingui();
  const size = row.size;
  return row.container === "array"
    ? t`${plural(size, { one: "# элемент", few: "# элемента", many: "# элементов", other: "# элемента" })}`
    : t`${plural(size, { one: "# ключ", few: "# ключа", many: "# ключей", other: "# ключа" })}`;
}

function Label({ row }: { row: JsonRow }) {
  if (row.label === null) {
    return null;
  }
  return (
    <>
      <span className="text-(--code-key)">
        {row.isIndex ? row.label : JSON.stringify(row.label)}
      </span>
      <Punct>{": "}</Punct>
    </>
  );
}

function RowBody({ row }: { row: JsonRow }) {
  const { open, close } = brackets(row);
  const comma = row.comma ? <Punct>{","}</Punct> : null;
  if (row.kind === "close") {
    return (
      <Punct>
        {close}
        {comma}
      </Punct>
    );
  }
  if (row.kind === "open") {
    return (
      <>
        <Label row={row} />
        <Punct>{open}</Punct>
      </>
    );
  }
  if (row.kind === "collapsed") {
    return (
      <>
        <Label row={row} />
        <Punct>{`${open}…${close}`}</Punct>
        <span className="ml-2 text-muted-foreground">
          <Summary row={row} />
        </span>
        {comma}
      </>
    );
  }
  return (
    <>
      <Label row={row} />
      {row.container === undefined ? (
        <span className={scalarClass(row.value)}>{scalarText(row.value)}</span>
      ) : (
        <Punct>{`${open}${close}`}</Punct>
      )}
      {comma}
    </>
  );
}

function Line({
  row,
  number,
  onToggle,
}: {
  row: JsonRow;
  number: number;
  onToggle: (nodeId: string) => void;
}) {
  const expanded = row.kind === "open";
  const toggleable = expanded || row.kind === "collapsed";
  const name = row.label ?? ROOT_NAME;
  const handleToggle = useCallback(() => onToggle(row.nodeId), [onToggle, row.nodeId]);
  return (
    <div className="flex min-h-6 items-start hover:bg-(--row-hover-bg)">
      <span
        aria-hidden="true"
        className="box-content w-[4ch] shrink-0 pr-3 text-right text-caption leading-6 text-(--code-line-no) tabular-nums select-none"
      >
        {number}
      </span>
      <div
        className="flex min-w-0 flex-1 items-start"
        style={{ paddingLeft: `${row.depth * INDENT_REM}rem` }}
      >
        {toggleable ? (
          <Button
            variant="ghost"
            size="icon-sm"
            aria-expanded={expanded}
            aria-label={`${expanded ? COLLAPSE : EXPAND} ${name}`}
            onClick={handleToggle}
          >
            {expanded ? (
              <ChevronDownIcon aria-hidden="true" />
            ) : (
              <ChevronRightIcon aria-hidden="true" />
            )}
          </Button>
        ) : (
          <span aria-hidden="true" className="size-6 shrink-0" />
        )}
        <span className="min-w-0 flex-1 font-mono text-caption leading-6 wrap-anywhere">
          <RowBody row={row} />
        </span>
      </div>
    </div>
  );
}

/** Дерево JSON со сворачиванием узлов и номерами строк. Значения выводятся текстом, разметка из данных невозможна. */
export function JsonViewer({ value, label = REGION, className }: JsonViewerProps) {
  const [collapsed, setCollapsed] = useState<ReadonlySet<string>>(() => defaultCollapsed(value));
  const rows = useMemo(() => flatten(value, collapsed), [value, collapsed]);
  const toggle = useCallback((nodeId: string) => {
    setCollapsed((prev) => {
      const next = new Set(prev);
      if (!next.delete(nodeId)) {
        next.add(nodeId);
      }
      return next;
    });
  }, []);
  const expandAll = useCallback(() => setCollapsed(new Set()), []);
  const collapseEverything = useCallback(() => setCollapsed(collapseAll(value)), [value]);
  return (
    <div className={cx("flex min-w-0 flex-col gap-2", className)}>
      <div className="flex flex-wrap items-center gap-2">
        <Button variant="ghost" size="sm" onClick={expandAll}>
          <ChevronsUpDownIcon aria-hidden="true" />
          {EXPAND_ALL}
        </Button>
        <Button variant="ghost" size="sm" onClick={collapseEverything}>
          <ChevronsDownUpIcon aria-hidden="true" />
          {COLLAPSE_ALL}
        </Button>
        <CopyButton value={JSON.stringify(value, null, JSON_INDENT)} label={COPY_JSON} />
      </div>
      <section
        aria-label={label}
        // biome-ignore lint/a11y/noNoninteractiveTabindex: область с прокруткой доступна с клавиатуры (WCAG 2.1.1)
        tabIndex={0}
        className="max-h-[min(70vh,720px)] overflow-auto rounded-lg bg-(--code-bg) py-2"
      >
        {rows.map((row, index) => (
          <Line key={row.rowId} row={row} number={index + 1} onToggle={toggle} />
        ))}
      </section>
    </div>
  );
}
