import type { MessageDescriptor } from "@lingui/core";
import { i18n } from "~/shared/lib/i18n";

/**
 * Перевод текста интерфейса экрана в момент рендера: `tr(msg\`Закрыть\`)`. Глобальный `t` Lingui устарел,
 * а хук недоступен в ячейках и фабриках, поэтому макет берёт активную локаль напрямую.
 */
export function tr(descriptor: MessageDescriptor): string {
  return i18n._(descriptor);
}
