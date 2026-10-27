import type { UserConfig } from "@commitlint/types";

// Conventional Commits + запрет атрибуции ИИ-агентов (правила продукта, CLAUDE.md).
const AI_ATTRIBUTION =
  /co-authored-by:.*(claude|anthropic|copilot|chatgpt|openai|gemini|cursor|codex|\bai\b)|generated (with|by).*(claude|ai\b|copilot|chatgpt)|noreply@anthropic\.com|🤖/i;

const RELEASE_COMMIT = /^chore\(release\): v\d+\.\d+\.\d+/;
const HEADER_MAX_LENGTH = 100;

const config: UserConfig = {
  extends: ["@commitlint/config-conventional"],
  plugins: [
    {
      rules: {
        "no-ai-attribution": ({ raw }) => [
          !AI_ATTRIBUTION.test(raw ?? ""),
          "атрибуция ИИ-агентов (Co-Authored-By, «Generated with …», 🤖) запрещена",
        ],
      },
    },
  ],
  rules: {
    "no-ai-attribution": [2, "always"],
    "type-enum": [
      2,
      "always",
      [
        "feat",
        "fix",
        "perf",
        "refactor",
        "docs",
        "test",
        "build",
        "ci",
        "chore",
        "revert",
        "style",
      ],
    ],
    "scope-case": [2, "always", "kebab-case"],
    "subject-case": [0],
    "header-max-length": [2, "always", HEADER_MAX_LENGTH],
    "body-max-line-length": [0],
    "footer-max-line-length": [0],
  },
  ignores: [(message: string) => RELEASE_COMMIT.test(message)],
};

export default config;
