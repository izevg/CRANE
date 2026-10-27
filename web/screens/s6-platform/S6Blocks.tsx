import { msg } from "@lingui/core/macro";
import {
  FlameIcon,
  FlaskConicalIcon,
  type LucideIcon,
  ShieldCheckIcon,
  Trash2Icon,
} from "lucide-react";
import type { ReactNode } from "react";
import { fn } from "storybook/test";
import { i18n } from "~/shared/lib/i18n";
import { type Action, ActionButton } from "~/shared/ui/actions";
import { Badge, Card, Link } from "~/shared/ui/primitives";
import { StatusBadge } from "~/shared/ui/status";
import type { TopRow } from "./S6Rows";

/** Спека волны 2: §3 — entities/platform HealthList. */
type Health = {
  name: string;
  tone: "success" | "warning" | "error";
  label: string;
  caption: string;
  href: string;
};

// Сначала проблемные (patterns §3).
function components(): readonly Health[] {
  return [
    {
      name: "Upstream npmjs",
      tone: "error",
      label: i18n._(msg`Недоступен`),
      caption: i18n._(msg`с 13:42 · клиенты получают кэш до 7 дней`),
      href: "/platform/upstreams",
    },
    {
      name: i18n._(msg`LLM-шлюз`),
      tone: "warning",
      label: i18n._(msg`Деградация`),
      caption: i18n._(msg`таймаут 12 с · включена резервная модель`),
      href: "/platform/integrations",
    },
    {
      name: "PostgreSQL",
      tone: "success",
      label: i18n._(msg`Работает`),
      caption: i18n._(msg`p95 1,8 мс`),
      href: "/status",
    },
    {
      name: "Valkey",
      tone: "success",
      label: i18n._(msg`Работает`),
      caption: i18n._(msg`p95 0,4 мс`),
      href: "/status",
    },
    {
      name: "S3 main",
      tone: "success",
      label: i18n._(msg`Работает`),
      caption: i18n._(msg`p95 22 мс`),
      href: "/status",
    },
    {
      name: "Trivy",
      tone: "success",
      label: i18n._(msg`Работает`),
      caption: i18n._(msg`база v2026.10.27`),
      href: "/platform/integrations",
    },
  ];
}

export function HealthList() {
  return (
    <ul className="flex flex-col">
      {components().map((item) => (
        <li key={item.name} className="border-b last:border-b-0">
          <Link
            href={item.href}
            className="flex min-h-11 items-center justify-between gap-3 py-1.5 hover:bg-accent"
          >
            <span className="flex min-w-0 flex-col">
              <span className="text-body font-semibold">{item.name}</span>
              <span className="text-caption text-muted-foreground">{item.caption}</span>
            </span>
            <StatusBadge tone={item.tone} label={item.label} />
          </Link>
        </li>
      ))}
    </ul>
  );
}

/** Спека волны 2: §32 — shared/ui/layout QuickActions. */
type Quick = { icon: LucideIcon; action: Action; caption: string };

function quick(): readonly Quick[] {
  return [
    {
      icon: FlaskConicalIcon,
      action: {
        label: i18n._(msg`Пробный прогон retention`),
        run: { onSelect: fn() },
      },
      caption: i18n._(msg`Покажет кандидатов на удаление; ничего не удаляет`),
    },
    {
      icon: Trash2Icon,
      action: {
        label: i18n._(msg`Применить retention`),
        run: { onSelect: fn() },
        stepUp: true,
      },
      caption: i18n._(msg`Удалит 1 284 версии (212 ГБ) без возможности отката`),
    },
    {
      icon: FlameIcon,
      action: {
        label: i18n._(msg`Прогреть кэш по lockfile`),
        run: { href: "/platform/readiness" },
      },
      caption: i18n._(msg`Поставит загрузку зависимостей в очередь`),
    },
    {
      icon: ShieldCheckIcon,
      action: {
        label: i18n._(msg`Проверить целостность`),
        run: { onSelect: fn() },
      },
      caption: i18n._(msg`Запустит асинхронную проверку, откроется E7`),
    },
  ];
}

export function QuickActions() {
  return (
    <ul className="flex flex-col gap-3">
      {quick().map(({ icon: Icon, action, caption }) => (
        <li key={action.label} className="flex items-start gap-3">
          <Icon aria-hidden="true" className="mt-2 size-5 shrink-0 text-muted-foreground" />
          <div className="flex min-w-0 flex-col items-start gap-1">
            <ActionButton action={action} variant="secondary" />
            <span className="text-caption text-muted-foreground">{caption}</span>
          </div>
        </li>
      ))}
    </ul>
  );
}

export function Panel({
  title,
  count,
  children,
  className,
}: {
  title: string;
  count?: string;
  children: ReactNode;
  className?: string;
}) {
  return (
    <Card className={className}>
      <section className="flex flex-col gap-3 px-4">
        <div className="flex items-center justify-between gap-2">
          <h2 className="text-h3">{title}</h2>
          {count ? <Badge variant="secondary">{count}</Badge> : null}
        </div>
        {children}
      </section>
    </Card>
  );
}

/** Спека волны 2: §31 — shared/ui/layout TopThreeSummary (h2 · бейдж-счётчик · ≤ 3 строки · ссылка). */

export function TopThree({
  rows,
  link,
}: {
  rows: readonly TopRow[];
  link: { href: string; label: string };
}) {
  return (
    <>
      <ul className="flex flex-col">
        {rows.map((row) => (
          <li
            key={row.id}
            className="flex items-start justify-between gap-3 border-b py-2 last:border-b-0"
          >
            <div className="flex min-w-0 flex-col">
              <span className="text-body font-semibold wrap-anywhere">{row.title}</span>
              <span className="text-caption text-muted-foreground wrap-anywhere">
                {row.caption}
              </span>
            </div>
            {row.badge ? <Badge variant="outline">{row.badge}</Badge> : null}
            {row.action ? <ActionButton action={row.action} variant="ghost" /> : null}
          </li>
        ))}
      </ul>
      <Link href={link.href} className="w-fit text-body font-semibold text-brand hover:underline">
        {link.label}
      </Link>
    </>
  );
}
