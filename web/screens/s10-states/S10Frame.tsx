import { useLingui } from "@lingui/react/macro";
import type { ComponentProps } from "react";
import { ScreenFrame } from "../frame/ScreenFrame";

/** Крошки строятся при рендере: текст зависит от активной локали, а `args` читаются при загрузке модуля. */
export function Frame(props: ComponentProps<typeof ScreenFrame>) {
  const { t } = useLingui();
  const breadcrumbs = {
    trail: [{ label: t`Каталог`, href: "/search" }],
    current: t`Мои пакеты`,
  };
  return <ScreenFrame {...props} breadcrumbs={breadcrumbs} />;
}
