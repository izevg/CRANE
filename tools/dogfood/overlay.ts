// Overlay манифестов: превращает копию воркспейса CRANE в публикуемую в hosted-реестр. Настоящие
// манифесты остаются прежними (path-зависимости без реестра), правится только копия.
import { parse, stringify } from "smol-toml";

type Table = Record<string, unknown>;

const DEPENDENCY_TABLES = ["dependencies", "dev-dependencies", "build-dependencies"];

const isTable = (value: unknown): value is Table =>
  typeof value === "object" && value !== null && !Array.isArray(value);

/** Поле таблицы TOML по имени из переменной: точечный доступ к индексной сигнатуре tsc не пропускает. */
const field = (holder: Table, key: string): unknown => holder[key];

/** Внутренней зависимости с `path` добавляются `version` и `registry`; `path` снимет cargo при упаковке. */
function rewriteDependencies(
  dependencies: unknown,
  internal: ReadonlySet<string>,
  version: string,
  registry: string,
): void {
  if (!isTable(dependencies)) {
    return;
  }
  for (const [key, spec] of Object.entries(dependencies)) {
    if (!isTable(spec) || typeof field(spec, "path") !== "string") {
      continue;
    }
    // `package = "..."` переименовывает зависимость: внутренность определяется настоящим именем пакета.
    const renamed = field(spec, "package");
    const name = typeof renamed === "string" ? renamed : key;
    if (internal.has(name)) {
      Object.assign(spec, { version, registry });
    }
  }
}

/**
 * Манифест, пригодный для `cargo publish --registry <registry>`: у каждой path-зависимости на внутренний
 * крейт (в dependencies, dev-, build- и в target.*) появляются `version` и `registry`; у корня воркспейса
 * `workspace.package.publish = [registry]`. Остальное не меняется.
 */
export function publishable(
  manifest: string,
  internal: ReadonlySet<string>,
  version: string,
  registry: string,
): string {
  const doc = parse(manifest);
  const rewrite = (holder: unknown): void => {
    if (!isTable(holder)) {
      return;
    }
    for (const name of DEPENDENCY_TABLES) {
      rewriteDependencies(holder[name], internal, version, registry);
    }
  };
  rewrite(doc);
  const targets = field(doc, "target");
  if (isTable(targets)) {
    for (const platform of Object.values(targets)) {
      rewrite(platform);
    }
  }
  const workspace = field(doc, "workspace");
  const shared = isTable(workspace) ? field(workspace, "package") : undefined;
  if (isTable(shared)) {
    Object.assign(shared, { publish: [registry] });
  }
  return stringify(doc);
}
