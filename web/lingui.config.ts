import { defineConfig } from "@lingui/cli";
import { formatter } from "@lingui/format-po";

export default defineConfig({
  sourceLocale: "ru",
  locales: ["ru", "en"],
  // Без цепочки запасных локалей: иначе EN молча показал бы русский текст и `lingui check missing` был бы зелёным зря.
  fallbackLocales: false,
  catalogs: [
    {
      path: "<rootDir>/src/shared/lib/i18n/locales/{locale}",
      include: ["<rootDir>/src/"],
      exclude: ["**/*.stories.tsx", "**/*.test.ts", "**/*.test.tsx"],
    },
  ],
  // Без номеров строк: запись в каталоге меняется только вместе с самим сообщением, а не при сдвиге кода.
  format: formatter({ origins: false, lineNumbers: false }),
});
