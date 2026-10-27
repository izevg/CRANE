import { msg } from "@lingui/core/macro";
import { useLingui } from "@lingui/react/macro";
import { useCallback, useState } from "react";
import { i18n } from "~/shared/lib/i18n";
import { FilterSelect, SegmentedControl } from "~/shared/ui/inputs";
import { Button, Input } from "~/shared/ui/primitives";
import { activeFormat } from "../frame/format";

function periods() {
  return [
    {
      value: "24h",
      label: i18n._(msg`Последние 24 часа`),
    },
    {
      value: "7d",
      label: i18n._(msg`Последние 7 дней`),
    },
    {
      value: "30d",
      label: i18n._(msg`Последние 30 дней`),
    },
  ] as const;
}
const ACTIONS = [
  { value: "package.version.blocked", label: "package.version.blocked" },
  { value: "token.issued", label: "token.issued" },
  { value: "policy.cedar.updated", label: "policy.cedar.updated" },
  { value: "audit.revealed", label: "audit.revealed" },
] as const;
function decisions() {
  return [
    {
      value: "allow",
      label: i18n._(msg`Разрешено`),
    },
    {
      value: "deny",
      label: i18n._(msg`Отказ`),
    },
    {
      value: "challenge",
      label: i18n._(msg`Нужен MFA`),
    },
  ] as const;
}
const CHANNELS = ["UI", "CLI", "API", "CI", "A2A", "SOAR"].map((value) => ({
  value,
  label: value,
}));
function actorKinds() {
  return [
    {
      value: "all",
      label: i18n._(msg`Все`),
    },
    {
      value: "human",
      label: i18n._(msg`Человек`),
    },
    {
      value: "robot",
      label: i18n._(msg`Робот`),
    },
    { value: "workload", label: "Workload" },
    {
      value: "agent",
      label: i18n._(msg`Агент`),
    },
  ] as const;
}
type ActorKind = ReturnType<typeof actorKinds>[number]["value"];

// Спека волны 2: S9 фильтры (screens.md) — будущий feature `AuditFilters` (DateRangePicker с пресетами ещё нет).
export function Filters() {
  const { t } = useLingui();
  const [period, setPeriod] = useState<string | null>("24h");
  const [action, setAction] = useState<string | null>(null);
  const [decision, setDecision] = useState<string | null>(null);
  const [channel, setChannel] = useState<string | null>(null);
  const [kind, setKind] = useState<ActorKind>("all");
  const changeKind = useCallback((next: ActorKind) => setKind(next), []);
  return (
    <div className="flex flex-col gap-3">
      <div className="flex flex-wrap items-center gap-2">
        <FilterSelect
          label={t`Период`}
          allLabel={t`Всё время`}
          value={period}
          options={periods()}
          onValueChange={setPeriod}
        />
        <Input
          aria-label={t`Актор (псевдоним)`}
          placeholder={t`Актор (псевдоним)`}
          className="max-w-52"
        />
        <FilterSelect
          label={t`Действие`}
          allLabel={t`Все действия`}
          value={action}
          options={ACTIONS}
          onValueChange={setAction}
        />
        <FilterSelect
          label={t`Решение`}
          allLabel={t`Любое решение`}
          value={decision}
          options={decisions()}
          onValueChange={setDecision}
        />
      </div>
      <div className="flex flex-wrap items-center gap-2">
        <SegmentedControl
          label={t`Тип актора`}
          items={actorKinds()}
          value={kind}
          onValueChange={changeKind}
        />
        <FilterSelect
          label={t`Канал`}
          allLabel={t`Все каналы`}
          value={channel}
          options={CHANNELS}
          onValueChange={setChannel}
        />
        <Input
          aria-label={t`request_id или ресурс`}
          placeholder={t`request_id или ресурс`}
          className="max-w-56"
        />
        <Input aria-label={t`IP-подсеть`} placeholder={t`IP-подсеть`} className="max-w-40" />
        <div className="ml-auto flex items-center gap-3">
          <Button variant="ghost" size="sm">
            {t`Сбросить`}
          </Button>
          <span className="text-body text-muted-foreground tabular-nums">
            {activeFormat().count({ count: 18_420, capped: false })} {t`событий`}
          </span>
        </div>
      </div>
    </div>
  );
}
