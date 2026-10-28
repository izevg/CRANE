import type { MessageDescriptor } from "@lingui/core";
import { msg } from "@lingui/core/macro";
import { ActivityIcon, type LucideIcon } from "lucide-react";
import { type HalLinks, halHref } from "~/shared/api";
import { i18n } from "~/shared/lib/i18n";
import type { SidebarProps } from "~/widgets/app-shell";

type GroupId = "platform";

/** Пункт навигации: отношение `_links` из `GET /me` и маршрут приложения, на который оно ведёт. */
export type NavSpec = {
  rel: string;
  to: string;
  label: MessageDescriptor;
  icon: LucideIcon;
  group: GroupId;
};

const GROUP_LABEL = { platform: msg`Платформа` } satisfies Record<GroupId, MessageDescriptor>;
const GROUP_ORDER: readonly GroupId[] = ["platform"];

/**
 * Соответствие rel → маршрут живёт в app: кит и API о маршрутах не знают. Новый раздел добавляет строку сюда
 * вместе со своим маршрутом. `crane:status` — раздел, доступный всем ролям.
 */
const NAV: readonly NavSpec[] = [
  {
    rel: "crane:status",
    to: "/status",
    label: msg`Статус системы`,
    icon: ActivityIcon,
    group: "platform",
  },
];

type Context = {
  /** Есть ли у роутера такой маршрут. */
  hasRoute: (to: string) => boolean;
  /** Совпадает ли текущее положение роутера с маршрутом (источник `aria-current`). */
  isActive: (to: string) => boolean;
};

/**
 * Группы сайдбара из `_links`. Пункт показывается, только если ссылка есть в ответе (раздел разрешён) и у
 * роутера есть маршрут: ссылка без маршрута привела бы на 404. Названия ролей в выборе не участвуют.
 */
export function navGroups(
  links: HalLinks,
  context: Context,
  spec: readonly NavSpec[] = NAV,
): SidebarProps["groups"] {
  const groups: SidebarProps["groups"][number][] = [];
  for (const id of GROUP_ORDER) {
    const items = spec
      .filter(
        (item) =>
          item.group === id && halHref(links, item.rel) !== undefined && context.hasRoute(item.to),
      )
      .map((item) => ({
        id: item.rel,
        label: i18n._(item.label),
        href: item.to,
        icon: item.icon,
        active: context.isActive(item.to),
      }));
    if (items.length > 0) {
      groups.push({ id, label: i18n._(GROUP_LABEL[id]), items });
    }
  }
  return groups;
}
