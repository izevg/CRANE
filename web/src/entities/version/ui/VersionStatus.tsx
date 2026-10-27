import { useLingui } from "@lingui/react/macro";
import { BLOCK_REASON_KIND, VERSION_STATE, type VersionSummary } from "~/shared/api";
import { Badge } from "~/shared/ui/primitives";
import { StatusBadge } from "~/shared/ui/status";
import { reasonView } from "../lib/blockReason";

type VersionStatusProps = {
  version: Pick<VersionSummary, "state" | "block_reason">;
};

const SEPARATOR = " · ";

/**
 * Состояние версии: бейдж из словаря, а у заблокированной рядом причина («Экстренно · CVE-2026-31337»), чтобы она
 * не пряталась в флагах (patterns §9). Ближайшая исправленная версия — видимым текстом рядом: на касании подсказки нет.
 */
export function VersionStatus({ version }: VersionStatusProps) {
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
      {fixedIn === undefined ? null : (
        <span className="text-caption text-muted-foreground">{t`Исправлено в ${fixedIn}`}</span>
      )}
    </span>
  );
}
