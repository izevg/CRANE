@AGENTS.md

## Claude Code

- All project rules are in `AGENTS.md` (imported above). This file holds only what is specific to Claude Code.
- Attribution in commits and PRs is disabled at the project level (`.claude/settings.json`: `attribution.commit` and `attribution.pr` are empty). Do not add `Co-Authored-By` or "Generated with" by hand.
- The `ROADMAP.md` milestones are mandatory (see "Roadmap" in `AGENTS.md`). Work in a worktree: the `EnterWorktree` tool (directory `.claude/worktrees/`) or `claude --worktree`; do not edit files in the main directory. Clean up worktrees of merged branches as described in "Worktrees" in `AGENTS.md` (`task worktrees:prune`).
- Before committing, run the checks from the commands table; the `lefthook` hooks run on their own, do not bypass them.

## Code graphs (graft, graphify)

Both graphs are local and gitignored; build them once per worktree: `graft build` and `graphify update .` (AST only, no API key).

- Navigate through the graphs before raw grep or reading whole files: `graft ask "<question>"`, `graft skeleton <file>` (API surface), `graft callers <symbol>`, `graft map`; `graphify query "<question>"`, `graphify path "<A>" "<B>"`, `graphify explain "<concept>"`.
- Before changing a public item, check its blast radius: `graft callers <symbol> --depth all` or `graft blast`.
- After modifying code, refresh the graphs: `graft build` and `graphify update .`.
