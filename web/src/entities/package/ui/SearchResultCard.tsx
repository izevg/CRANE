import { plural } from "@lingui/core/macro";
import { useLingui } from "@lingui/react/macro";
import { useId } from "react";
import {
  FORMAT,
  LIFECYCLE_STATE,
  PRIORITY,
  REPOSITORY_KIND,
  type SearchItem,
  type SearchRisk,
  type Typosquat,
  VERSION_STATE,
} from "~/shared/api";
import { Badge, Callout, Link } from "~/shared/ui/primitives";
import { RelativeTime, StatusBadge } from "~/shared/ui/status";

type SearchResultCardProps = {
  item: SearchItem;
  /** Страница пакета (маршрут SPA): единственная ссылка карточки. `_links` API здесь не читается. */
  href: string;
  /**
   * Страница настоящего пакета (маршрут SPA). Карточка `_links` не читает: страница строит маршрут из
   * `similarToHref(item)` и не передаёт проп, если ссылки `crane:similar-to` нет. Нет пропа — имя и PURL текстом.
   */
  typosquatHref?: string;
};

function TyposquatNotice({ typosquat, href }: { typosquat: Typosquat; href: string | undefined }) {
  const { t } = useLingui();
  const { similar_to: name, similar_to_purl: purl, distance } = typosquat;
  return (
    <Callout
      tone="warning"
      className="flex w-fit max-w-full flex-wrap items-baseline gap-x-1 py-1 text-caption"
    >
      <span>{t`Похоже на`}</span>
      {href === undefined ? (
        <span className="wrap-anywhere">{name}</span>
      ) : (
        <Link href={href} className="underline wrap-anywhere">
          {name}
        </Link>
      )}
      <span className="wrap-anywhere text-muted-foreground">{`(${purl})`}</span>
      <span>
        {t`— отличается на ${plural(distance, {
          one: "# символ",
          few: "# символа",
          many: "# символов",
          other: "# символа",
        })} (возможный typosquat)`}
      </span>
    </Callout>
  );
}

function RiskPill({ risk }: { risk: SearchRisk }) {
  const { t } = useLingui();
  const priority = risk.max_open_priority;
  const count = risk.open_count;
  return (
    <StatusBadge
      {...PRIORITY[priority]}
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
  const { latest, downloads_30d: downloads, owner_team: team, last_release_at: released } = item;
  return (
    <article
      aria-labelledby={titleId}
      className="flex min-w-0 flex-col gap-1.5 rounded-xl bg-card p-4 shadow-card"
    >
      {item.exact_match ? <p className="text-overline text-brand">{t`Точное совпадение`}</p> : null}
      {item.typosquat == null ? null : (
        <TyposquatNotice typosquat={item.typosquat} href={typosquatHref} />
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
        {item.lifecycle.state === "deprecated" ? (
          <StatusBadge {...LIFECYCLE_STATE[item.lifecycle.state]} />
        ) : null}
        {item.risk ? (
          <span className="ml-auto">
            <RiskPill risk={item.risk} />
          </span>
        ) : null}
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
