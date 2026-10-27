const LOCALE = "ru-RU";

const formatters = new Map<string, Intl.NumberFormat>();

function formatter(options: Intl.NumberFormatOptions): Intl.NumberFormat {
  const key = JSON.stringify(options);
  let cached = formatters.get(key);
  if (cached === undefined) {
    cached = new Intl.NumberFormat(LOCALE, options);
    formatters.set(key, cached);
  }
  return cached;
}

/** `48102` → «48 102» (разделитель групп — неразрывный пробел). */
export function formatNumber(value: number, options: Intl.NumberFormatOptions = {}): string {
  return formatter(options).format(value);
}

/** Доля 0..1 → «94,2 %» (один знак после запятой, неразрывный пробел перед знаком). */
export function formatPercent(ratio: number): string {
  return formatter({ style: "percent", minimumFractionDigits: 1, maximumFractionDigits: 1 }).format(
    ratio,
  );
}

type CountValue = {
  count: number;
  /** Сервер ограничил подсчёт: реальное число не меньше `count`. */
  capped: boolean;
};

/** `{1000, capped}` → «1 000+»: усечённый счётчик нельзя показывать как точный. */
export function formatCount({ count, capped }: CountValue): string {
  return capped ? `${formatNumber(count)}+` : formatNumber(count);
}

const BYTE_UNITS = ["byte", "kilobyte", "megabyte", "gigabyte", "terabyte", "petabyte"] as const;
const BYTES_STEP = 1024;

const NBSP = "\u00a0";

/**
 * Размер в байтах → «18,4 ТБ». Ступень 1024, как в реестрах артефактов; единицы — по правилам Intl.
 * Пробел перед единицей делается неразрывным: число не отрывается от единицы при переносе.
 */
export function formatBytes(bytes: number): string {
  let value = bytes;
  let unit = 0;
  while (Math.abs(value) >= BYTES_STEP && unit < BYTE_UNITS.length - 1) {
    value /= BYTES_STEP;
    unit += 1;
  }
  return formatter({
    style: "unit",
    unit: BYTE_UNITS[unit] ?? "byte",
    unitDisplay: "short",
    maximumFractionDigits: unit === 0 ? 0 : 1,
  })
    .format(value)
    .replaceAll(" ", NBSP);
}
