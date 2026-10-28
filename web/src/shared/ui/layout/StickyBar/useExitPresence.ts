import { useEffect, useRef, useState } from "react";

type Presence<T> = {
  mounted: boolean;
  /** Выход идёт: элемент ещё на экране, но уже должен быть неактивным и прозрачным. */
  leaving: boolean;
  /** Текущее значение, а на выходе — последнее живое: панель не показывает нули, пока уезжает. */
  value: T | undefined;
};

/**
 * Держит элемент смонтированным `exitMs` после того, как `value` стал `null`: так отрабатывает анимация выхода.
 * Пока значение есть, оно же и возвращается; на выходе возвращается последнее непустое.
 */
export function useExitPresence<T>(value: T | null, exitMs: number): Presence<T> {
  const live = value !== null;
  const [mounted, setMounted] = useState(live);
  const last = useRef<T | undefined>(undefined);
  if (value !== null) {
    last.current = value;
  }
  useEffect(() => {
    if (live) {
      setMounted(true);
      return;
    }
    const timer = setTimeout(() => setMounted(false), exitMs);
    return () => clearTimeout(timer);
  }, [live, exitMs]);
  return { mounted: live || mounted, leaving: !live && mounted, value: last.current };
}
