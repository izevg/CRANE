import { plural } from "@lingui/core/macro";
import { useLingui } from "@lingui/react/macro";
import { useId } from "react";
import {
  FORMAT,
  LIFECYCLE_STATE,
  REPOSITORY_KIND,
  type SearchItem,
  VERSION_STATE,
} from "~/shared/api";
import { Badge, Callout, Link } from "~/shared/ui/primitives";
import { RelativeTime, StatusBadge } from "~/shared/ui/status";
import {
  type RiskPill as RiskPillData,
  riskPill,
  showsLifecycleBadge,
  typosquatOf,
} from "../lib/card";

type SearchResultCardProps = {
  item: SearchItem;
  /** Страница пакета (маршрут SPA): единственная ссылка карточки. `_links` API здесь не читается. */
  href: string;
  /** Страница настоящего пакета для предупреждения о typosquat; без неё имя показывается без ссылки. */
  typosquatHref?: string;
};

function TyposquatNotice({
  similarTo,
  distance,
  href,
}: {
  similarTo: string;
  distance: number;
  href: string | undefined;
}) {
  const { t } = useLingui();
  return (
    <Callout tone="warning" className="py-1 text-caption">
      {t`${plural(distance, {
        one: `Похоже на ${similarTo} — # символ (возможный typosquat).`,
        few: `Похоже на ${similarTo} — # символа (возможный typosquat).`,
        many: `Похоже на ${similarTo} — # символов (возможный typosquat).`,
        other: `Похоже на ${similarTo} — # символа (возможный typosquat).`,
      })}`}{" "}
      {href === undefined ? null : (
        <Link href={href} className="underline">
          {similarTo}
        </Link>
      )}
    </Callout>
  );
}

function RiskPill({ priority, count, tone }: RiskPillData) {
  const { t } = useLingui();
  return (
    <StatusBadge
      tone={tone}
      label={t`${plural(count, {
        one: `${priority} · # находка`,
        few: `${priority} · # находки`,
        many: `${priority} · # находок`,
        other: `${priority} · # находки`,
      })}`}
    />
  );
}

/**
 * Карточка результата поиска (patterns §6). Ссылка в ней одна, на имени пакета: карточка не обёрнута в `<a>`
 * и не содержит вложенных кнопок. Пилюли «Чисто» нет: риск рисуется, только если он есть.
 */
export function SearchResultCard({ item, href, typosquatHref }: SearchResultCardProps) {
  const { t } = useLingui();
  const titleId = useId();
  const risk = riskPill(item);
  const typosquat = typosquatOf(item);
  const { latest, downloads_30d: downloads, owner_team: team, last_release_at: released } = item;
  return (
    <article
      aria-labelledby={titleId}
      className="flex min-w-0 flex-col gap-1.5 rounded-xl bg-card p-4 shadow-card"
    >
      {item.exact_match ? <p className="text-overline text-brand">{t`Точное совпадение`}</p> : null}
      {typosquat === null ? null : (
        <TyposquatNotice
          similarTo={typosquat.similarTo}
          distance={typosquat.distance}
          href={typosquatHref}
        />
      )}
      <div className="flex flex-wrap items-center gap-x-2 gap-y-1">
        <h3 id={titleId} className="min-w-0 text-h3 wrap-anywhere">
          <Link href={href} className="text-foreground hover:text-brand">
            {item.name.display}
          </Link>
        </h3>
        <Badge variant="outline">{FORMAT[item.format]}</Badge>
        <Badge variant="secondary">{`${item.repository.name} · ${item.repository.workspace}`}</Badge>
        <Badge variant="outline">{REPOSITORY_KIND[item.repository.kind]}</Badge>
        {showsLifecycleBadge(item) ? (
          <StatusBadge {...LIFECYCLE_STATE[item.lifecycle.state]} />
        ) : null}
        {risk === null ? null : (
          <span className="ml-auto">
            <RiskPill priority={risk.priority} count={risk.count} tone={risk.tone} />
          </span>
        )}
      </div>
      {item.description === null ? null : (
        <p className="truncate text-body text-muted-foreground">{item.description}</p>
      )}
      <p className="flex flex-wrap items-center gap-x-3 gap-y-1 text-caption text-muted-foreground">
        {latest === null ? null : (
          <span className="flex items-center gap-2">
            <span className="inline-flex gap-1">
              <span>{t`Последняя`}</span> <span className="font-mono">{latest.version}</span>
            </span>
            <StatusBadge {...VERSION_STATE[latest.state]} />
          </span>
        )}
        {downloads === null ? null : (
          <span className="tabular-nums">
            {t`${plural(downloads, {
              one: "# загрузка за 30 дн",
              few: "# загрузки за 30 дн",
              many: "# загрузок за 30 дн",
              other: "# загрузки за 30 дн",
            })}`}
          </span>
        )}
        {team === null ? null : <span>{team}</span>}
        {released === null ? null : <RelativeTime ts={released} />}
      </p>
    </article>
  );
}
