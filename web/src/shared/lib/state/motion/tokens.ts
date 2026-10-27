/** Значения совпадают с `shared/styles/motion.css` (проверяется тестом). Длительности — в миллисекундах. */
export const durInstant = 100;
export const durFast = 150;
export const durBase = 200;
export const durSlow = 250;
export const durMax = 300;

const MS_IN_SECOND = 1000;

/** Motion принимает секунды. */
export const toSeconds = (ms: number): number => ms / MS_IN_SECOND;
