import { useLingui } from "@lingui/react/macro";
import { useCallback, useEffect, useRef, useState } from "react";
import { announce } from "~/shared/lib/a11y";
import { type Action, ActionButton } from "~/shared/ui/actions";
import { Button, Callout } from "~/shared/ui/primitives";
import { announcementFor, hiddenCount, visibleBanners } from "../../lib/banners";

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
  const { t } = useLingui();
  const { id, text } = banner;
  const handleClick = useCallback(() => onCollapse(id), [id, onCollapse]);
  return (
    <Button variant="ghost" size="sm" aria-label={t`Свернуть: ${text}`} onClick={handleClick}>
      {t`Свернуть`}
    </Button>
  );
}

function BannerItem({ banner, onCollapse }: { banner: Banner; onCollapse: Props["onCollapse"] }) {
  return (
    <Callout tone={banner.tone} {...(banner.urgent === true ? { live: "alert" as const } : {})}>
      <div className="flex flex-wrap items-start justify-between gap-x-4 gap-y-1">
        <span className="min-w-0 flex-1">{banner.text}</span>
        {/* -my-1.5: кнопка 32 px не растягивает строку выше 20 px и остаётся на уровне первой строки текста */}
        <div className="-my-1.5 flex items-center gap-2">
          {banner.action ? <ActionButton action={banner.action} variant="ghost" /> : null}
          {banner.tone === "info" && onCollapse !== undefined ? (
            <CollapseButton banner={banner} onCollapse={onCollapse} />
          ) : null}
        </div>
      </div>
    </Callout>
  );
}

/**
 * Новый обычный баннер объявляется один раз через общий live-регион (WCAG 4.1.3, patterns §25): сами баннеры
 * без live-роли, иначе скринридер читал бы их при каждом монтировании. Срочные (P0) объявляет `role="alert"`
 * у Callout, поэтому сюда они не попадают. Id, пропавший из набора, забывается и при возвращении объявится снова.
 */
function useAnnounceNew(banners: readonly Banner[]) {
  const seen = useRef(new Set<string>());
  useEffect(() => {
    const next = announcementFor(banners, seen.current);
    seen.current = next.seen;
    if (next.text !== null) {
      announce(next.text);
    }
  }, [banners]);
}

/** Глобальные баннеры под topbar: видно не больше двух, остальные раскрываются кнопкой «Ещё N». */
export function BannerStack({ banners, onCollapse }: Props) {
  const { t } = useLingui();
  const [expanded, setExpanded] = useState(false);
  useAnnounceNew(banners);
  const toggle = useCallback(() => setExpanded((value) => !value), []);
  if (banners.length === 0) {
    return null;
  }
  const hidden = hiddenCount(banners.length);
  const shown = visibleBanners(banners, expanded);
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
          {expanded ? t`Скрыть остальные` : t`Ещё ${hidden}`}
        </Button>
      ) : null}
    </div>
  );
}
