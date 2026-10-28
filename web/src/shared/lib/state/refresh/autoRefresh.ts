import { createFlag } from "../flag/flag";

/**
 * Пауза автообновления — один флаг на приложение (WCAG 2.2.2): его переключает AutoRefreshToggle, а опрос операций
 * (`shared/api`) читает. Живёт в памяти: после перезагрузки опрос снова идёт, как задумано контрактом.
 */
export const autoRefreshPaused = createFlag();
