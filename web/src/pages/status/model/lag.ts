const SECONDS_PER_MINUTE = 60;

export type Lag = { value: number; unit: "s" | "min" };

/** Отставание в удобной единице: до минуты — секунды, дальше — целые минуты. `null` — данных нет. */
export function lagOf(seconds: number | null | undefined): Lag | null {
  if (seconds === null || seconds === undefined) {
    return null;
  }
  return seconds < SECONDS_PER_MINUTE
    ? { value: seconds, unit: "s" }
    : { value: Math.round(seconds / SECONDS_PER_MINUTE), unit: "min" };
}
