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
  assert.equal(decide(wt, [merged()], ctx).remove, true);
});

it("открытый или закрытый без слияния PR не удаляется", () => {
  assert.equal(decide(wt, [merged({ state: "OPEN" })], ctx).remove, false);
  assert.equal(decide(wt, [merged({ state: "CLOSED" })], ctx).remove, false);
  assert.equal(decide(wt, [], ctx).remove, false);
});

it("PR, влитый в чужую ветку, не считается релизом", () => {
  assert.equal(decide(wt, [merged({ baseRefName: "feat/other" })], ctx).remove, false);
});

it("новые коммиты после слияния сохраняют worktree", () => {
  assert.equal(decide({ ...wt, head: "def" }, [merged()], ctx).remove, false);
});

it("грязный, текущий, основной, detached и защищённые не удаляются", () => {
  assert.equal(decide(wt, [merged()], { ...ctx, dirty: true }).remove, false);
  assert.equal(decide(wt, [merged()], { ...ctx, isCurrent: true }).remove, false);
  assert.equal(decide(wt, [merged()], { ...ctx, isMain: true }).remove, false);
  assert.equal(decide({ ...wt, branch: null }, [merged()], ctx).remove, false);
  assert.equal(decide({ ...wt, branch: "release/v0.1.0" }, [merged()], ctx).remove, false);
  assert.equal(decide({ ...wt, branch: "main" }, [merged()], ctx).remove, false);
});

it("porcelain разбирается в worktree", () => {
  const out =
    "worktree /r\nHEAD 111\nbranch refs/heads/main\n\nworktree /r/w\nHEAD 222\ndetached\n";
  assert.deepEqual(parseWorktrees(out), [
    { path: "/r", head: "111", branch: "main" },
    { path: "/r/w", head: "222", branch: null },
  ]);
});
