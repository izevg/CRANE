export type JsonValue =
  | string
  | number
  | boolean
  | null
  | readonly JsonValue[]
  | { readonly [key: string]: JsonValue };

type Container = readonly JsonValue[] | { readonly [key: string]: JsonValue };

type RowBase = {
  /** Уникальный ключ строки; у закрывающей скобки отличается от ключа открывающей. */
  rowId: string;
  /** Идентификатор узла, стабилен между свёртками. */
  nodeId: string;
  depth: number;
  /** Ключ объекта или индекс массива; у корня `null`. */
  label: string | null;
  isIndex: boolean;
  comma: boolean;
};

type ContainerKind = "object" | "array";

/** Строка дерева. Поля зависят от `kind`, поэтому лишних необязательных свойств нет. */
export type JsonRow =
  | (RowBase & { kind: "leaf"; value: JsonValue; container?: ContainerKind })
  | (RowBase & { kind: "open"; container: ContainerKind; size: number })
  | (RowBase & { kind: "collapsed"; container: ContainerKind; size: number })
  | (RowBase & { kind: "close"; container: ContainerKind });

const ROOT_ID = "$";
/** Дерево короче этого числа строк разворачивается целиком. */
const FULL_LIMIT = 200;
const OPEN_DEPTH = 2;

function isContainer(value: JsonValue): value is Container {
  return typeof value === "object" && value !== null;
}

function entriesOf(value: Container): [string | number, JsonValue][] {
  return Array.isArray(value)
    ? value.map((item, index): [number, JsonValue] => [index, item])
    : Object.entries(value as { readonly [key: string]: JsonValue });
}

function childId(parent: string, key: string | number): string {
  return `${parent}/${String(key)}`;
}

/** Идентификаторы всех раскрываемых (непустых) контейнеров вместе с глубиной. */
function containerNodes(root: JsonValue): { nodeId: string; depth: number }[] {
  const out: { nodeId: string; depth: number }[] = [];
  const walk = (value: JsonValue, nodeId: string, depth: number) => {
    if (!isContainer(value)) {
      return;
    }
    const entries = entriesOf(value);
    if (entries.length === 0) {
      return;
    }
    out.push({ nodeId, depth });
    for (const [key, child] of entries) {
      walk(child, childId(nodeId, key), depth + 1);
    }
  };
  walk(root, ROOT_ID, 0);
  return out;
}

/** Число строк, если всё развёрнуто. */
function countRows(value: JsonValue): number {
  if (!isContainer(value)) {
    return 1;
  }
  const entries = entriesOf(value);
  if (entries.length === 0) {
    return 1;
  }
  let count = 2;
  for (const [, child] of entries) {
    count += countRows(child);
  }
  return count;
}

/** Короткие деревья раскрыты целиком; у длинных свёрнуты узлы глубже второго уровня. */
export function defaultCollapsed(root: JsonValue): ReadonlySet<string> {
  if (countRows(root) <= FULL_LIMIT) {
    return new Set();
  }
  return new Set(
    containerNodes(root)
      .filter((node) => node.depth > OPEN_DEPTH)
      .map((node) => node.nodeId),
  );
}

export function collapseAll(root: JsonValue): ReadonlySet<string> {
  return new Set(containerNodes(root).map((node) => node.nodeId));
}

type Walk = { label: string | null; isIndex: boolean; comma: boolean };

/** Разворачивает дерево в плоский список строк с учётом свёрнутых узлов. Вход не изменяется. */
export function flatten(root: JsonValue, collapsed: ReadonlySet<string>): JsonRow[] {
  const rows: JsonRow[] = [];

  const walk = (value: JsonValue, nodeId: string, depth: number, meta: Walk) => {
    const base = { nodeId, depth, ...meta };
    if (!isContainer(value)) {
      rows.push({ ...base, rowId: nodeId, kind: "leaf", value });
      return;
    }
    const container = Array.isArray(value) ? "array" : "object";
    const entries = entriesOf(value);
    if (entries.length === 0) {
      rows.push({ ...base, rowId: nodeId, kind: "leaf", value, container });
      return;
    }
    if (collapsed.has(nodeId)) {
      rows.push({ ...base, rowId: nodeId, kind: "collapsed", container, size: entries.length });
      return;
    }
    rows.push({
      ...base,
      rowId: nodeId,
      kind: "open",
      container,
      size: entries.length,
      comma: false,
    });
    for (const [index, [key, child]] of entries.entries()) {
      walk(child, childId(nodeId, key), depth + 1, {
        label: String(key),
        isIndex: typeof key === "number",
        comma: index < entries.length - 1,
      });
    }
    rows.push({ ...base, rowId: `${nodeId}:close`, kind: "close", container });
  };

  walk(root, ROOT_ID, 0, { label: null, isIndex: false, comma: false });
  return rows;
}
