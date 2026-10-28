import { useSyncExternalStore } from "react";
import { isAutoRefreshPaused, subscribeAutoRefresh } from "./autoRefresh";

/** Текущее значение флага паузы; компонент перерисовывается при переключении. */
export function useAutoRefreshPaused(): boolean {
  return useSyncExternalStore(subscribeAutoRefresh, isAutoRefreshPaused, isAutoRefreshPaused);
}
