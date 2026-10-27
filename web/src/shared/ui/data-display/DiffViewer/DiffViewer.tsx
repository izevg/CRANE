import { useMemo } from "react";
import { cx } from "~/shared/lib/core";

/** Строка диффа, посчитанного сервером. `path` — поле или номер строки, если он есть. */
export type DiffRow = { kind: "add" | "remove" | "same"; text: string; path?: string };

type DiffViewerProps = {
  rows: readonly DiffRow[];
  /** `inline` — подряд (по умолчанию, для панели); `split` — две колонки (для полной страницы). */
  mode?: "inline" | "split";
  /** Доступное имя таблицы: «Изменения манифеста 1.4.1 → 1.4.2». */
  label: string;
};

const BEFORE = "Было";
const AFTER = "Стало";
const PATH_HEAD = "Поле";
const CHANGE_HEAD = "Изменение";
const TEXT_HEAD = "Содержимое";

const MARKER = {
  add: { sign: "+", text: "добавлено" },
  remove: { sign: "−", text: "удалено" },
  same: { sign: "", text: "" },
} satisfies Record<DiffRow["kind"], { sign: string; text: string }>;

const TONE = {
  add: "bg-(--diff-add-bg) text-(--diff-add-fg)",
  remove: "bg-(--diff-del-bg) text-(--diff-del-fg)",
  same: "",
} satisfies Record<DiffRow["kind"], string>;

const CELL = "px-3 py-1 align-top font-mono text-caption whitespace-pre-wrap wrap-anywhere";
const PATH =
  "px-3 py-1 text-left align-top text-caption font-normal text-muted-foreground wrap-anywhere";
const SIGN_COLUMN = "w-8 px-2 py-1 text-center align-top font-mono text-caption font-semibold";

/** Знак + скрытый текст: изменение не передаётся только цветом (WCAG 1.4.1). */
function Marker({ kind }: { kind: DiffRow["kind"] }) {
  const { sign, text } = MARKER[kind];
  if (sign === "") {
    return null;
  }
  return (
    <>
      <span aria-hidden="true">{sign}</span>
      <span className="sr-only">{text}</span>
    </>
  );
}

function PathCell({ row }: { row: DiffRow }) {
  return (
    <th scope="row" className={PATH}>
      {row.path}
    </th>
  );
}

function InlineRow({ row, withPath }: { row: DiffRow; withPath: boolean }) {
  return (
    <tr className={TONE[row.kind]}>
      <td className={SIGN_COLUMN}>
        <Marker kind={row.kind} />
      </td>
      {withPath ? <PathCell row={row} /> : null}
      <td className={CELL}>{row.text}</td>
    </tr>
  );
}

/** Слева остаются неизменные и удалённые строки, справа неизменные и добавленные. */
function SplitRow({ row, withPath }: { row: DiffRow; withPath: boolean }) {
  const left = row.kind !== "add";
  const right = row.kind !== "remove";
  return (
    <tr>
      {withPath ? <PathCell row={row} /> : null}
      <td className={cx(CELL, left && TONE[row.kind])}>
        {left ? (
          <>
            <Marker kind={row.kind === "remove" ? "remove" : "same"} />
            {row.text}
          </>
        ) : null}
      </td>
      <td className={cx(CELL, right && TONE[row.kind])}>
        {right ? (
          <>
            <Marker kind={row.kind === "add" ? "add" : "same"} />
            {row.text}
          </>
        ) : null}
      </td>
    </tr>
  );
}

/**
 * Показывает готовый дифф: сравнение выполняет сервер. Добавление и удаление различаются знаком «+»/«−» и
 * скрытым текстом для скринридера, цвет лишь усиливает их.
 */
export function DiffViewer({ rows, mode = "inline", label }: DiffViewerProps) {
  const withPath = rows.some((row) => row.path !== undefined);
  const keyed = useMemo(
    () => rows.map((row, index) => ({ row, key: `${index}:${row.kind}` })),
    [rows],
  );
  const split = mode === "split";
  return (
    <div className="overflow-x-auto rounded-lg border">
      <table className="w-full table-fixed border-collapse text-body">
        <caption className="sr-only">{label}</caption>
        <thead
          className={
            split ? "bg-surface-high text-left text-overline text-muted-foreground" : "sr-only"
          }
        >
          <tr>
            {split ? null : (
              <th scope="col" className="w-8">
                {CHANGE_HEAD}
              </th>
            )}
            {withPath ? (
              <th scope="col" className="w-[28%] px-3 py-1.5 font-semibold">
                {PATH_HEAD}
              </th>
            ) : null}
            {split ? (
              <>
                <th scope="col" className="px-3 py-1.5 font-semibold">
                  {BEFORE}
                </th>
                <th scope="col" className="px-3 py-1.5 font-semibold">
                  {AFTER}
                </th>
              </>
            ) : (
              <th scope="col">{TEXT_HEAD}</th>
            )}
          </tr>
        </thead>
        <tbody>
          {keyed.map(({ row, key }) =>
            split ? (
              <SplitRow key={key} row={row} withPath={withPath} />
            ) : (
              <InlineRow key={key} row={row} withPath={withPath} />
            ),
          )}
        </tbody>
      </table>
    </div>
  );
}
