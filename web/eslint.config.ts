import type { Linter } from "eslint";
import sonarjs from "eslint-plugin-sonarjs";
import tseslint from "typescript-eslint";

const sonarRecommended = sonarjs.configs?.recommended;
if (sonarRecommended === undefined || Array.isArray(sonarRecommended)) {
  throw new Error("eslint-plugin-sonarjs: ожидался один объект configs.recommended");
}

// Предупреждений не бывает (как в Next.js): любой уровень поднимается до error,
// иначе предупреждения копятся и перестают читаться.
const strict = (config: Linter.Config): Linter.Config => ({
  ...config,
  rules: Object.fromEntries(
    Object.entries(config.rules ?? {}).map(([name, entry]) => {
      const [level, ...options] = Array.isArray(entry) ? entry : [entry];
      return [name, level === "off" || level === 0 ? entry : ["error", ...options]];
    }),
  ),
});

export default tseslint.config(
  { ignores: ["dist", "coverage", "reports", ".stryker-tmp"] },
  ...tseslint.configs.recommended.map((config) => strict(config as Linter.Config)),
  strict(sonarRecommended as Linter.Config),
  {
    files: ["**/*.{ts,tsx}"],
    linterOptions: { reportUnusedDisableDirectives: "error" },
  },
);
