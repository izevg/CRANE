import { useLingui } from "@lingui/react/macro";
import { BLOCK_REASON_KIND, VERSION_STATE, type VersionSummary } from "~/shared/api";
import { CopyableId } from "~/shared/ui/actions";
import { Badge, Link } from "~/shared/ui/primitives";
import { StatusBadge } from "~/shared/ui/status";

type VersionStatusProps = {
  version: Pick<VersionSummary, "state" | "block_reason">;
  /**
   * Маршрут страницы экстренной блокировки из `_links.crane:block`: страница читает HAL и строит маршрут, UI не
   * собирает его из `block_id`. Идентификатор становится ссылкой; без маршрута он остаётся копируемым значением.
   */
  blockHref?: string;
};

const SEPARATOR = " · ";

function BlockId({ id, href }: { id: string; href: string | undefined }) {
  const { t } = useLingui();
  return href === undefined ? (
    <CopyableId value={id} label={t`Скопировать идентификатор блокировки`} />
  ) : (
    <Link href={href} className="font-mono text-caption underline" aria-label={t`Блокировка ${id}`}>
      {id}
    </Link>
  );
}

/**
 * Состояние версии: бейдж из словаря, а у заблокированной рядом причина («Экстренно · CVE-2026-31337»), чтобы она
 * не пряталась в флагах (patterns §9). Ближайшая исправленная версия — видимым текстом рядом: на касании подсказки нет.
 */
export function VersionStatus({ version, blockHref }: VersionStatusProps) {
  const { t } = useLingui();
  // Причина показывается только у заблокированной версии; ветвление по `kind` — здесь, в одном месте
  const reason = version.state === "blocked" ? version.block_reason : null;
  const detail = reason?.kind === "emergency" ? reason.advisory_id : reason?.rule_id;
  const reasonText =
    reason === null ? null : [t(BLOCK_REASON_KIND[reason.kind]), detail].join(SEPARATOR);
  const fixedIn = reason?.fixed_in;
  return (
    <span className="flex flex-wrap items-center gap-2">
      <StatusBadge {...VERSION_STATE[version.state]} />
      {reasonText === null ? null : <Badge variant="destructive">{reasonText}</Badge>}
      {reason?.kind === "emergency" ? <BlockId id={reason.block_id} href={blockHref} /> : null}
      {fixedIn === undefined ? null : (
        <span className="text-caption text-muted-foreground">{t`Исправлено в ${fixedIn}`}</span>
      )}
    </span>
  );
}
