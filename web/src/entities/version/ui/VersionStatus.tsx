import { useLingui } from "@lingui/react/macro";
import { BLOCK_REASON_KIND, VERSION_STATE, type VersionSummary } from "~/shared/api";
import { CopyableId } from "~/shared/ui/actions";
import { Badge, Link } from "~/shared/ui/primitives";
import { StatusBadge } from "~/shared/ui/status";
import { reasonView } from "../lib/blockReason";

type VersionStatusProps = {
  version: Pick<VersionSummary, "state" | "block_reason">;
  /**
   * Страница экстренной блокировки (маршрут SPA, его строит страница): идентификатор становится ссылкой. Без
   * неё он остаётся копируемым машинным значением.
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
  const reason = reasonView(version.state, version.block_reason);
  const fixedIn = reason?.fixedIn;
  const reasonText =
    reason === null
      ? null
      : [t(BLOCK_REASON_KIND[reason.kind]), reason.detail].filter(Boolean).join(SEPARATOR);
  return (
    <span className="flex flex-wrap items-center gap-2">
      <StatusBadge {...VERSION_STATE[version.state]} />
      {reason === null ? null : <Badge variant="destructive">{reasonText}</Badge>}
      {reason?.blockId === undefined ? null : <BlockId id={reason.blockId} href={blockHref} />}
      {fixedIn === undefined ? null : (
        <span className="text-caption text-muted-foreground">{t`Исправлено в ${fixedIn}`}</span>
      )}
    </span>
  );
}
