/**
 * Игнорер Stryker для макросов Lingui. Мутанты внутри вызовов `t`, `msg`, `plural` и им подобных не создаются:
 * Stryker оборачивает литералы и блоки в тернарные выражения, и макрос уже не может раскрыть своё сообщение.
 * Остальной код тех же файлов мутируется как обычно, поэтому мутаторы BlockStatement, ObjectLiteral и StringLiteral
 * остаются включёнными. Игнорируются также тела функций с `useLingui()` (см. `declaresUseLingui`).
 *
 * Файл подключается как плагин Stryker (`plugins` в `web/stryker.config.json`) и включается именем в `ignorers`.
 * Типы Stryker не импортируются: плагин описывается обычным объектом, а `web` не зависит от `@stryker-mutator/api`.
 */

const MACRO_TAGS = new Set(["t", "msg", "defineMessage"]);
const MACRO_CALLS = new Set(["t", "msg", "defineMessage", "plural", "select", "selectOrdinal"]);
const REASON = "Макрос Lingui: инструментатор ломает его раскрытие";

type Named = { type: string; name?: string };

type BabelNode = {
  type: string;
  tag?: Named;
  callee?: Named;
  body?: readonly BabelStatement[];
};

type BabelStatement = {
  type: string;
  declarations?: readonly { init?: { type: string; callee?: Named } | null }[];
};

function isNamed(
  expression: { type: string; name?: string } | undefined,
  names: ReadonlySet<string>,
): boolean {
  return expression?.type === "Identifier" && names.has(expression.name ?? "");
}

/**
 * Тело функции с `useLingui()`. Макрос раскрывает хук только в теле функции; обёртка BlockStatement-мутанта
 * (`if (stryMutAct(..)) {} else { const { t } = useLingui(); ... }`) прячет объявление во вложенный блок, и раскрытие
 * не происходит. Логика таких компонентов живёт в чистых модулях без макросов и мутируется там.
 */
function declaresUseLingui(node: BabelNode): boolean {
  return (
    node.type === "BlockStatement" &&
    (node.body ?? []).some((statement) =>
      (statement.declarations ?? []).some(
        (declarator) =>
          declarator.init?.type === "CallExpression" &&
          declarator.init.callee?.name === "useLingui",
      ),
    )
  );
}

/** Причина игнорирования узла или `undefined`, если мутировать его можно. */
export function shouldIgnore(path: { node: BabelNode }): string | undefined {
  const { node } = path;
  if (node.type === "TaggedTemplateExpression" && isNamed(node.tag, MACRO_TAGS)) {
    return REASON;
  }
  if (node.type === "CallExpression" && isNamed(node.callee, MACRO_CALLS)) {
    return REASON;
  }
  if (declaresUseLingui(node)) {
    return REASON;
  }
  return undefined;
}

/** Ожидается загрузчиком плагинов Stryker (`PluginKind.Ignore` = "Ignore"). */
export const strykerPlugins = [{ kind: "Ignore", name: "lingui-macros", value: { shouldIgnore } }];
