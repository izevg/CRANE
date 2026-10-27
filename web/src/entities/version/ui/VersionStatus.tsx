import { useLingui } from "@lingui/react/macro";
import { BLOCK_REASON_KIND, VERSION_STATE, type VersionSummary } from "~/shared/api";
import { Badge, Tooltip, TooltipContent, TooltipTrigger } from "~/shared/ui/primitives";
import { StatusBadge } from "~/shared/ui/status";
import { reasonView } from "../lib/blockReason";

type VersionStatusProps = {
  version: Pick<VersionSummary, "state" | "block_reason">;
};

const SEPARATOR = " · ";

/**
 * Состояние версии: бейдж из словаря, а у заблокированной рядом причина («Экстренно · CVE-2026-31337»), чтобы она
 * не пряталась в флагах (patterns §9). Ближайшая исправленная версия — в подсказке.
 */
export function VersionStatus({ version }: VersionStatusProps) {
  const { t } = useLingui();
  const { tone, label } = VERSION_STATE[version.state];
  const reason = reasonView(version.state, version.block_reason);
  const fixedIn = reason?.fixedIn;
  const reasonText =
    reason === null
      ? null
      : [t(BLOCK_REASON_KIND[reason.kind]), reason.detail].filter(Boolean).join(SEPARATOR);
  return (
    <span className="flex flex-wrap items-center gap-2">
      <StatusBadge tone={tone} label={t(label)} />
      {reason === null ? null : fixedIn === undefined ? (
        <Badge variant="destructive">{reasonText}</Badge>
      ) : (
        <Tooltip>
          <TooltipTrigger render={<Badge variant="destructive" tabIndex={0} />}>
            {reasonText}
          </TooltipTrigger>
          <TooltipContent>{t`Исправлено в ${fixedIn}`}</TooltipContent>
        </Tooltip>
      )}
    </span>
  );
}
