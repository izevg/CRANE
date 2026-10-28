// Реестр заглушек: формат `FIXME(vX.Y.Z): текст`, привязка к вехам ROADMAP.md
// и к операциям контракта. Чистые функции, весь IO живёт в index.ts.

export type Hit = { file: string; line: number; text: string };
// next - первая значимая строка после заглушки; нужна только правилу shared/api.
export type Fixme = { file: string; line: number; version: string; text: string; next?: string };
export type Milestone = { version: string; done: boolean };
export type Operation = { id: string; since: string | undefined; source: string };
export type OperationIndex = { since: Map<string, string>; errors: string[] };

// Единственный допустимый маркер долга - FIXME; остальные запрещены.
export const MARKERS = ["FIXME", "TODO", "XXX", "HACK"] as const;
export const MARKERS_PATTERN = MARKERS.join("|");

export const WEB_SRC = "web/src/";
export const WEB_API = "web/src/shared/api/";

const FORMAT_HINT = "FIXME(vX.Y.Z): что осталось доделать";
const FORBIDDEN_MARKER = new RegExp(`\\b(${MARKERS.filter((m) => m !== "FIXME").join("|")})\\b`);
const FIXME_FORMAT = /FIXME\((v\d+\.\d+\.\d+)\): (\S.*)$/;
const MILESTONE_ROW = /^\|\s*(?:\*\*)?(v\d+\.\d+\.\d+)(?:\*\*)?\s*\|/;
const DONE_MARK = "(готово)";
const MILESTONES_HEADING = "## Вехи";
const NEXT_HEADING = "## ";
const OPERATION_ID = /^\s*operationId:\s*['"]?([^\s'"]+)['"]?\s*$/;
const CRANE_SINCE = /^\s*x-crane-since:\s*['"]?([^\s'"]+)['"]?\s*$/;
const GREP_LINE = /^([^:]+):(\d+):(.*)$/;
const COMMENT_OR_BLANK = /^\s*(?:$|\/\/|\/\*|\*)/;
const WHITESPACE = /\s+/;

/** Вывод `git grep -n`: `путь:строка:текст`, двоеточия в тексте остаются в тексте. */
export const parseGrep = (stdout: string): Hit[] =>
  stdout.split("\n").flatMap((raw) => {
    const [, file, line, text] = GREP_LINE.exec(raw) ?? [];
    return file === undefined || line === undefined || text === undefined
      ? []
      : [{ file, line: Number(line), text }];
  });

/** Разобранная заглушка либо сообщение о нарушении формата. */
export const parseFixme = (hit: Hit): Fixme | string => {
  const where = `${hit.file}:${hit.line}`;
  const forbidden = FORBIDDEN_MARKER.exec(hit.text)?.[1];
  if (forbidden !== undefined) {
    return `${where}: маркер ${forbidden} запрещён: замените на ${FORMAT_HINT} из реестра или уберите`;
  }
  const found = FIXME_FORMAT.exec(hit.text.trimEnd());
  const [, version, text] = found ?? [];
  if (version === undefined || text === undefined) {
    const shown = hit.text.trim();
    return `${where}: FIXME не по формату (${shown}). Нужно: ${FORMAT_HINT}, версия из таблицы «Вехи» ROADMAP.md`;
  }
  return { file: hit.file, line: hit.line, version, text: text.trim() };
};

/** Дописывает заглушке первую следующую строку, которая не пуста и не комментарий. */
export const withNext = (fixme: Fixme, source: string): Fixme => {
  const next = source
    .split("\n")
    .slice(fixme.line)
    .find((line) => !COMMENT_OR_BLANK.test(line));
  return next === undefined ? fixme : { ...fixme, next };
};

/** Только таблица раздела «Вехи»: порядок строк задаёт порядок вех. */
export const parseMilestones = (roadmap: string): Milestone[] => {
  const milestones: Milestone[] = [];
  let inSection = false;
  for (const line of roadmap.split("\n")) {
    if (line.startsWith(MILESTONES_HEADING)) {
      inSection = true;
      continue;
    }
    if (inSection && line.startsWith(NEXT_HEADING)) {
      break;
    }
    const version = inSection ? MILESTONE_ROW.exec(line)?.[1] : undefined;
    if (version !== undefined) {
      milestones.push({ version, done: line.includes(DONE_MARK) });
    }
  }
  return milestones;
};

/** Плоский список операций файла; since ищется до следующего operationId, а не на фиксированном расстоянии. */
export const parseOperations = (yaml: string, source: string): Operation[] => {
  const found: Operation[] = [];
  for (const line of yaml.split("\n")) {
    const id = OPERATION_ID.exec(line)?.[1];
    if (id !== undefined) {
      found.push({ id, since: undefined, source });
      continue;
    }
    const current = found.at(-1);
    const value = CRANE_SINCE.exec(line)?.[1];
    if (current !== undefined && value !== undefined && current.since === undefined) {
      current.since = value;
    }
  }
  return found;
};

/** Один проход по операциям всех контрактов: индекс id -> since и оба вида нарушений (нет since, дубликат). */
export const indexOperations = (operations: Operation[]): OperationIndex => {
  const sources = new Map<string, string>();
  const index: OperationIndex = { since: new Map(), errors: [] };
  for (const { id, since, source } of operations) {
    const first = sources.get(id);
    if (since === undefined) {
      index.errors.push(
        `${source}: у операции ${id} нет x-crane-since, контракт не разобран. Добавьте x-crane-since: vX.Y.Z`,
      );
    } else if (first !== undefined) {
      index.errors.push(
        `operationId ${id} повторяется (${first} и ${source}). operationId должен быть уникален`,
      );
    } else {
      index.since.set(id, since);
      sources.set(id, source);
    }
  }
  return index;
};

const checkApiStub = (fixme: Fixme, versions: string[], operations: OperationIndex): string[] => {
  const where = `${fixme.file}:${fixme.line}`;
  const operationId = fixme.text.split(WHITESPACE)[0] ?? "";
  const since = operations.since.get(operationId);
  if (since === undefined) {
    return [
      `${where}: «${operationId}» не operationId из docs/api/admin.openapi.yaml или common.yaml. В shared/api первым словом текста идёт operationId: FIXME(${fixme.version}): <operationId> что доделать`,
    ];
  }
  const sinceIndex = versions.indexOf(since);
  if (sinceIndex < 0) {
    return [
      `${where}: x-crane-since ${since} операции ${operationId} нет в таблице «Вехи» ROADMAP.md. Исправьте контракт`,
    ];
  }
  if (versions.indexOf(fixme.version) < sinceIndex) {
    return [
      `${where}: веха ${fixme.version} раньше x-crane-since ${since} операции ${operationId}. Укажите ${since} или позже`,
    ];
  }
  // Версия в комментарии и в записи заглушки (её читает рантайм) не должна разъехаться.
  if (!fixme.next?.includes(`"${fixme.version}"`)) {
    return [
      `${where}: версия FIXME не совпадает с записью заглушки на следующей строке. Строка после комментария должна содержать литерал "${fixme.version}", например: ${operationId}: { replacedIn: "${fixme.version}", ... }`,
    ];
  }
  return [];
};

const checkFixme = (
  fixme: Fixme,
  milestones: Milestone[],
  operations: OperationIndex,
): string[] => {
  const where = `${fixme.file}:${fixme.line}`;
  const versions = milestones.map((m) => m.version);
  const milestone = milestones.find((m) => m.version === fixme.version);
  if (milestone === undefined) {
    return [
      `${where}: веха ${fixme.version} не найдена в таблице «Вехи» ROADMAP.md. Укажите версию из таблицы`,
    ];
  }
  if (milestone.done) {
    return [
      `${where}: веха ${fixme.version} закрыта, заглушка не снята. Реализуйте её и удалите FIXME либо перенесите на открытую веху`,
    ];
  }
  if (!fixme.file.startsWith(WEB_SRC)) {
    return [];
  }
  if (!fixme.file.startsWith(WEB_API)) {
    return [
      `${where}: FIXME в web/src допустим только под ${WEB_API}. Здесь заглушку не оставляйте: доделайте код или заведите её в shared/api`,
    ];
  }
  return checkApiStub(fixme, versions, operations);
};

export const check = (
  fixmes: Fixme[],
  milestones: Milestone[],
  contract: Operation[],
): string[] => {
  const operations = indexOperations(contract);
  return [
    // Пустая таблица означала бы, что каждая заглушка «неизвестна»: лучше одно внятное сообщение.
    ...(milestones.length === 0
      ? ["ROADMAP.md: в разделе «Вехи» не найдено ни одной строки `| vX.Y.Z |`"]
      : []),
    ...operations.errors,
    ...fixmes.flatMap((fixme) => checkFixme(fixme, milestones, operations)),
  ];
};

const NO_STUBS = "заглушек нет";
const NO_MILESTONE = "без вехи";

/** Группы в порядке таблицы; всё, что не привязалось к вехе, идёт отдельным блоком, а не теряется. */
export const report = (fixmes: Fixme[], invalid: string[], milestones: Milestone[]): string[] => {
  const known = new Set(milestones.map((m) => m.version));
  const lines: string[] = [];
  for (const { version } of milestones) {
    const group = fixmes.filter((f) => f.version === version);
    if (group.length > 0) {
      lines.push(
        `${version}: ${group.length}`,
        ...group.map((f) => `  ${f.file}:${f.line}  ${f.text}`),
      );
    }
  }
  const orphans = [
    ...invalid,
    ...fixmes
      .filter((f) => !known.has(f.version))
      .map((f) => `${f.file}:${f.line}: FIXME(${f.version}) - вехи нет в таблице: ${f.text}`),
  ];
  if (orphans.length > 0) {
    lines.push(`${NO_MILESTONE}: ${orphans.length}`, ...orphans.map((o) => `  ${o}`));
  }
  return lines.length > 0 ? lines : [NO_STUBS];
};
