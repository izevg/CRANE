import type { MessageDescriptor } from "@lingui/core";
import { useLingui } from "@lingui/react";
import { useMemo } from "react";
import { cx } from "~/shared/lib/core";
import {
  DIFF_ADDED,
  DIFF_AFTER,
  DIFF_BEFORE,
  DIFF_CHANGE,
  DIFF_CONTENT,
  DIFF_PATH,
  DIFF_REMOVED,
  diffChanged,
} from "../text/messages";

/** Строка диффа, посчитанного сервером. `path` — поле или номер строки, если он есть. */
export type DiffRow =
  | { kind: "add" | "remove" | "same"; text: string; path?: string }
  /** Изменённое поле: одна строка со старым и новым значением, а не пара «удалено + добавлено». */
  | { kind: "change"; before: string; after: string; path?: string };

type LineKind = "add" | "remove" | "same";

type DiffViewerProps = {
  rows: readonly DiffRow[];
  /** `inline` — подряд (по умолчанию, для панели); `split` — две колонки (для полной страницы). */
  mode?: "inline" | "split";
  /** Доступное имя таблицы: «Изменения манифеста 1.4.1 → 1.4.2». */
  label: string;
};

const MARKER = {
  add: { sign: "+", text: DIFF_ADDED },
  remove: { sign: "−", text: DIFF_REMOVED },
  same: { sign: "", text: null },
} satisfies Record<LineKind, { sign: string; text: MessageDescriptor | null }>;

const TONE = {
  add: "bg-(--diff-add-bg) text-(--diff-add-fg)",
  remove: "bg-(--diff-del-bg) text-(--diff-del-fg)",
  same: "",
} satisfies Record<LineKind, string>;

const CELL = "px-3 py-1 align-top font-mono text-caption whitespace-pre-wrap wrap-anywhere";
const PATH =
  "px-3 py-1 text-left align-top text-caption font-normal text-muted-foreground wrap-anywhere";

const SIGN_COLUMN = "w-8 px-2 py-1 text-center align-top font-mono text-caption font-semibold";

/** Знак + скрытый текст: изменение не передаётся только цветом (WCAG 1.4.1). */
function Marker({ kind }: { kind: LineKind }) {
  const { _ } = useLingui();
  const { sign, text } = MARKER[kind];
  if (text === null) {
    return null;
  }
  return (
    <>
      <span aria-hidden="true">{sign}</span>
      <span className="sr-only">{_(text)}</span>
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

/** Для скринридера изменённое поле читается одной фразой; визуальные строки скрыты (`aria-hidden`), чтобы не дублировать. */
function ChangeSummary({ before, after }: { before: string; after: string }) {
  const { _ } = useLingui();
  return <span className="sr-only">{_(diffChanged(before, after))}</span>;
}

function InlineRow({ row, withPath }: { row: DiffRow; withPath: boolean }) {
  if (row.kind === "change") {
    return (
      <tr>
        <td className={SIGN_COLUMN}>
          <span aria-hidden="true" className="flex flex-col">
            <span className={TONE.remove}>{MARKER.remove.sign}</span>
            <span className={TONE.add}>{MARKER.add.sign}</span>
          </span>
        </td>
        {withPath ? <PathCell row={row} /> : null}
        <td className="p-0">
          <ChangeSummary before={row.before} after={row.after} />
          <span aria-hidden="true" className="flex flex-col">
            <span className={cx(CELL, TONE.remove)}>{row.before}</span>
            <span className={cx(CELL, TONE.add)}>{row.after}</span>
          </span>
        </td>
      </tr>
    );
  }
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

/** Слева остаются неизменные и удалённые строки, справа неизменные и добавленные; изменённое поле занимает одну строку. */
function SplitRow({ row, withPath }: { row: DiffRow; withPath: boolean }) {
  if (row.kind === "change") {
    return (
      <tr>
        {withPath ? <PathCell row={row} /> : null}
        <td className={cx(CELL, TONE.remove)}>
          <ChangeSummary before={row.before} after={row.after} />
          <span aria-hidden="true">
            {MARKER.remove.sign}
            {row.before}
          </span>
        </td>
        <td aria-hidden="true" className={cx(CELL, TONE.add)}>
          {MARKER.add.sign}
          {row.after}
        </td>
      </tr>
    );
  }
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
  const { _ } = useLingui();
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
                {_(DIFF_CHANGE)}
              </th>
            )}
            {withPath ? (
              <th scope="col" className="w-[28%] px-3 py-1.5 font-semibold">
                {_(DIFF_PATH)}
              </th>
            ) : null}
            {split ? (
              <>
                <th scope="col" className="px-3 py-1.5 font-semibold">
                  {_(DIFF_BEFORE)}
                </th>
                <th scope="col" className="px-3 py-1.5 font-semibold">
                  {_(DIFF_AFTER)}
                </th>
              </>
            ) : (
              <th scope="col">{_(DIFF_CONTENT)}</th>
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
