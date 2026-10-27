import assert from "node:assert/strict";
import { test as it } from "node:test";
import {
  type Context,
  decide,
  type PullRequest,
  parseWorktrees,
  type Worktree,
} from "./prune-worktrees.ts";

const wt: Worktree = { path: "/r/.claude/worktrees/x", head: "abc", branch: "feat/x" };
const ctx: Context = { isMain: false, isCurrent: false, dirty: false };
const merged = (over: Partial<PullRequest> = {}): PullRequest => ({
  number: 7,
  state: "MERGED",
  baseRefName: "release/v0.1.0",
  headRefOid: "abc",
  ...over,
});

it("ветка влитого в релиз PR удаляется", () => {
  // Arrange
  const prs = [merged()];

  // Act
  const decision = decide(wt, prs, ctx);

  // Assert
  assert.equal(decision.remove, true);
});

const kept = [
  { name: "открытый PR", wt, prs: [merged({ state: "OPEN" })], ctx },
  { name: "закрытый без слияния PR", wt, prs: [merged({ state: "CLOSED" })], ctx },
  { name: "ветка без PR", wt, prs: [], ctx },
  { name: "PR, влитый в чужую ветку", wt, prs: [merged({ baseRefName: "feat/other" })], ctx },
  { name: "новые коммиты после слияния", wt: { ...wt, head: "def" }, prs: [merged()], ctx },
  { name: "грязное дерево", wt, prs: [merged()], ctx: { ...ctx, dirty: true } },
  { name: "текущий worktree", wt, prs: [merged()], ctx: { ...ctx, isCurrent: true } },
  { name: "основной worktree", wt, prs: [merged()], ctx: { ...ctx, isMain: true } },
  { name: "detached HEAD", wt: { ...wt, branch: null }, prs: [merged()], ctx },
  { name: "ветка релиза", wt: { ...wt, branch: "release/v0.1.0" }, prs: [merged()], ctx },
  { name: "ветка main", wt: { ...wt, branch: "main" }, prs: [merged()], ctx },
];

for (const c of kept) {
  it(`worktree не удаляется: ${c.name}`, () => {
    // Arrange

    // Act
    const decision = decide(c.wt, c.prs, c.ctx);

    // Assert
    assert.equal(decision.remove, false);
  });
}

it("porcelain разбирается в worktree", () => {
  // Arrange
  const out =
    "worktree /r\nHEAD 111\nbranch refs/heads/main\n\nworktree /r/w\nHEAD 222\ndetached\n";

  // Act
  const actual = parseWorktrees(out);

  // Assert
  assert.deepEqual(actual, [
    { path: "/r", head: "111", branch: "main" },
    { path: "/r/w", head: "222", branch: null },
  ]);
});
