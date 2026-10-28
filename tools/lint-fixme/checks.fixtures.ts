import type { Fixme, Milestone, Operation } from "./checks.ts";

export const MILESTONES: Milestone[] = [
  { version: "v0.2.0", done: true },
  { version: "v0.3.0", done: false },
  { version: "v0.4.0", done: false },
  { version: "v0.5.0", done: false },
];

export const OPERATIONS: Operation[] = [
  { id: "getMe", since: "v0.4.0", source: "admin.openapi.yaml" },
];

export const STUB_NEXT = 'getMe: { replacedIn: "v0.4.0", load: () => import("./me") },';

export const fixme = (overrides: Partial<Fixme>): Fixme => ({
  file: "crates/app/src/lib.rs",
  line: 7,
  version: "v0.4.0",
  text: "что угодно",
  ...overrides,
});
