import { useCallback, useState } from "react";
import { type Action, ActionButton } from "~/shared/ui/actions";
import { Button, Callout } from "~/shared/ui/primitives";

/** Больше двух баннеров одновременно — шум в глобальной зоне (patterns §25). */
const MAX_VISIBLE = 2;
const COLLAPSE = "Свернуть";
const MORE_PREFIX = "Ещё";
const HIDE_REST = "Скрыть остальные";

export type Banner = {
  id: string;
  tone: "error" | "warning" | "info";
  /** Что случилось и что это значит для пользователя, без техножаргона и ПДн. */
  text: string;
  /** Одна ссылка-действие справа. */
  action?: Action;
  /** P0: `role="alert"`, прерывает чтение. */
  urgent?: true;
};

type Props = {
  /** Уже отсортирован по важности: первый — самый важный. */
  banners: readonly Banner[];
  /** Сворачивать можно только info; error и warning не закрываются, пока состояние активно. */
  onCollapse?: ((id: string) => void) | undefined;
};

function CollapseButton({
  banner,
  onCollapse,
}: {
  banner: Banner;
  onCollapse: (id: string) => void;
}) {
  const { id, text } = banner;
  const handleClick = useCallback(() => onCollapse(id), [id, onCollapse]);
  return (
    <Button variant="ghost" size="sm" aria-label={`${COLLAPSE}: ${text}`} onClick={handleClick}>
      {COLLAPSE}
    </Button>
  );
}

function BannerItem({ banner, onCollapse }: { banner: Banner; onCollapse: Props["onCollapse"] }) {
  return (
    <Callout tone={banner.tone} urgent={banner.urgent === true}>
      <div className="flex flex-wrap items-center justify-between gap-x-4 gap-y-1">
        <span className="min-w-0 flex-1">{banner.text}</span>
        <div className="flex items-center gap-2">
          {banner.action ? <ActionButton action={banner.action} variant="ghost" /> : null}
          {banner.tone === "info" && onCollapse !== undefined ? (
            <CollapseButton banner={banner} onCollapse={onCollapse} />
          ) : null}
        </div>
      </div>
    </Callout>
  );
}

/** Глобальные баннеры под topbar: видно не больше двух, остальные раскрываются кнопкой «Ещё N». */
export function BannerStack({ banners, onCollapse }: Props) {
  const [expanded, setExpanded] = useState(false);
  const toggle = useCallback(() => setExpanded((value) => !value), []);
  if (banners.length === 0) {
    return null;
  }
  const hidden = banners.length - MAX_VISIBLE;
  const shown = expanded ? banners : banners.slice(0, MAX_VISIBLE);
  return (
    <div data-slot="banner-stack" className="flex flex-col gap-2 px-4 pt-4 lg:px-6 2xl:px-8">
      {shown.map((banner) => (
        <BannerItem key={banner.id} banner={banner} onCollapse={onCollapse} />
      ))}
      {hidden > 0 ? (
        <Button
          variant="ghost"
          size="sm"
          aria-expanded={expanded}
          className="self-start"
          onClick={toggle}
        >
          {expanded ? HIDE_REST : `${MORE_PREFIX} ${hidden}`}
        </Button>
      ) : null}
    </div>
  );
}
