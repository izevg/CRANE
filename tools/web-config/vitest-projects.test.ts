import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { test as it } from "node:test";
import { fileURLToPath } from "node:url";

const web = resolve(dirname(fileURLToPath(import.meta.url)), "../../web");
const CONFIGS = ["vitest.unit.config.ts", "vitest.config.ts"];
const GROUP_ORDER = /sequence:\s*\{\s*groupOrder:\s*(\d+)\s*\}/g;
const MAX_WORKERS = /maxWorkers:\s*\d+/g;

const read = (name: string): string => readFileSync(resolve(web, name), "utf8");
const all = (pattern: RegExp): string[] =>
  CONFIGS.flatMap((name) => [...read(name).matchAll(pattern)].map((m) => m[1] ?? m[0]));

// Vitest отказывается стартовать, если у проектов разный maxWorkers, а groupOrder один и тот же.
// Конфликт при rebase легко теряет эту строку молча, поэтому её наличие и уникальность проверяются здесь.
it("у каждого проекта с maxWorkers свой groupOrder", () => {
  // Arrange
  const orders = all(GROUP_ORDER);
  const capped = all(MAX_WORKERS);

  // Act
  const unique = new Set(orders);

  // Assert
  assert.ok(capped.length >= 2, "в обоих проектах должен быть maxWorkers");
  assert.equal(orders.length, capped.length, "groupOrder задан не у каждого проекта с maxWorkers");
  assert.equal(unique.size, orders.length, "groupOrder должен быть уникальным");
});
