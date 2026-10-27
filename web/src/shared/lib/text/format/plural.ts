const rules = new Intl.PluralRules("ru");

/** Склонение по `Intl.PluralRules("ru")`: `pluralRu(5, ["версия", "версии", "версий"])` → «версий». */
export function pluralRu(
  count: number,
  forms: readonly [one: string, few: string, many: string],
): string {
  const category = rules.select(Math.abs(count));
  if (category === "one") {
    return forms[0];
  }
  if (category === "few") {
    return forms[1];
  }
  return forms[2];
}
