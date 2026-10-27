import type { VersionFlags } from "~/shared/api";

export type FlagItem = {
  id: string;
  /** Термин протокола формата (yanked, latest…): не переводится. */
  label: string;
  /** Подсказка: у `deprecated` — сообщение автора. */
  hint: string | undefined;
};

/** Флаги формата в порядке показа: статусные, затем dist-tags. Пустой список — блока флагов нет. */
export function flagItems(flags: VersionFlags): readonly FlagItem[] {
  const items: FlagItem[] = [];
  if (flags.yanked) {
    items.push({ id: "yanked", label: "yanked", hint: undefined });
  }
  if (flags.unlisted) {
    items.push({ id: "unlisted", label: "unlisted", hint: undefined });
  }
  if (flags.deprecated_message !== null) {
    items.push({ id: "deprecated", label: "deprecated", hint: flags.deprecated_message });
  }
  if (flags.mutable) {
    items.push({ id: "mutable", label: "mutable", hint: undefined });
  }
  for (const tag of flags.dist_tags) {
    items.push({ id: `tag:${tag}`, label: tag, hint: undefined });
  }
  return items;
}
