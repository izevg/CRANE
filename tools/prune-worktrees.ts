// Очистка worktree веток, чей PR влит в релиз или main. По умолчанию dry-run; удаление: --apply.
// Использование: node tools/prune-worktrees.ts [--apply]
import { execFileSync } from "node:child_process";
import { realpathSync } from "node:fs";

export interface Worktree {
  path: string;
  head: string;
  branch: string | null;
}

export interface PullRequest {
  number: number;
  state: string;
  baseRefName: string;
  headRefOid: string;
}

export interface Verdict {
  remove: boolean;
  reason: string;
}

export interface Context {
  isMain: boolean;
  isCurrent: boolean;
  dirty: boolean;
}

export function parseWorktrees(porcelain: string): Worktree[] {
  return porcelain
    .split("\n\n")
    .filter((block) => block.trim() !== "")
    .map((block) => {
      const field = (key: string): string =>
        block
          .split("\n")
          .find((l) => l.startsWith(`${key} `))
          ?.slice(key.length + 1) ?? "";
      const ref = field("branch");
      return {
        path: field("worktree"),
        head: field("HEAD"),
        branch: ref.startsWith("refs/heads/") ? ref.slice("refs/heads/".length) : null,
      };
    });
}

const isProtected = (branch: string): boolean => branch === "main" || branch.startsWith("release/");
const isTarget = (base: string): boolean => base === "main" || base.startsWith("release/");

/** Удалять можно только worktree, чья ветка полностью влита PR в main или release/*. */
export function decide(wt: Worktree, prs: PullRequest[], ctx: Context): Verdict {
  if (ctx.isMain) return { remove: false, reason: "основной каталог" };
  if (ctx.isCurrent) return { remove: false, reason: "текущий worktree" };
  if (wt.branch === null) return { remove: false, reason: "detached HEAD" };
  if (isProtected(wt.branch)) return { remove: false, reason: "защищённая ветка" };
  if (ctx.dirty) return { remove: false, reason: "есть незакоммиченные изменения" };
  const merged = prs.filter((pr) => pr.state === "MERGED" && isTarget(pr.baseRefName));
  if (merged.length === 0) return { remove: false, reason: "PR не влит в main или release/*" };
  if (!merged.some((pr) => pr.headRefOid === wt.head)) {
    return { remove: false, reason: "после слияния PR в ветке есть новые коммиты" };
  }
  return { remove: true, reason: `PR #${merged[0]?.number ?? "?"} влит` };
}

const git = (args: string[], cwd?: string): string =>
  execFileSync("git", args, { encoding: "utf8", ...(cwd ? { cwd } : {}) });

function mergedPulls(branch: string): PullRequest[] {
  const out = execFileSync(
    "gh",
    [
      "pr",
      "list",
      "--head",
      branch,
      "--state",
      "merged",
      "--limit",
      "20",
      "--json",
      "number,state,baseRefName,headRefOid",
    ],
    { encoding: "utf8" },
  );
  return JSON.parse(out) as PullRequest[];
}

function main(): void {
  const apply = process.argv.includes("--apply");
  const here = realpathSync(process.cwd());
  const worktrees = parseWorktrees(git(["worktree", "list", "--porcelain"]));
  for (const [index, wt] of worktrees.entries()) {
    const ctx: Context = {
      isMain: index === 0,
      isCurrent: realpathSync(wt.path) === here,
      dirty: index !== 0 && git(["status", "--porcelain"], wt.path).trim() !== "",
    };
    const prs = wt.branch && !ctx.isMain && !ctx.isCurrent ? mergedPulls(wt.branch) : [];
    const verdict = decide(wt, prs, ctx);
    const label = wt.branch ?? wt.head.slice(0, 7);
    process.stdout.write(
      `${verdict.remove ? "удалить" : "оставить"}  ${label}  (${verdict.reason})\n`,
    );
    if (verdict.remove && apply && wt.branch) {
      git(["worktree", "remove", wt.path]);
      git(["branch", "-D", wt.branch]);
    }
  }
  if (!apply) process.stdout.write("dry-run: для удаления добавьте --apply\n");
}

if (import.meta.main) main();
