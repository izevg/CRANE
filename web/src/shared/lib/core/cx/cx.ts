/**
 * Склейка классов без разрешения конфликтов (в отличие от tailwind-merge).
 * В Tailwind v4 при конфликте побеждает порядок правил в CSS, а не порядок в строке, поэтому
 * `className` у компонентов кита предназначен только для размещения (margin, width, grid-area),
 * а варианты задаются картами `Record<Variant, string>`.
 */
export function cx(...parts: readonly (string | false | null | undefined)[]): string {
  return parts.filter((part): part is string => typeof part === "string" && part !== "").join(" ");
}

/**
 * Пропсы обёртки Base UI с `className` только строкой: функция от состояния в ките не нужна.
 * Дистрибутивен по объединениям: `Omit` без этого схлопнул бы дискриминированный union пропсов в один тип.
 */
export type WithClassName<P> = P extends unknown
  ? Omit<P, "className"> & { className?: string }
  : never;
