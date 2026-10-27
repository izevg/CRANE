@AGENTS.md

## Claude Code

- All project rules are in `AGENTS.md` (imported above). This file holds only what is specific to Claude Code.
- Attribution in commits and PRs is disabled at the project level (`.claude/settings.json`: `attribution.commit` and `attribution.pr` are empty). Do not add `Co-Authored-By` or "Generated with" by hand.
- The `ROADMAP.md` milestones are mandatory (see "Roadmap" in `AGENTS.md`). Work in a worktree: the `EnterWorktree` tool (directory `.claude/worktrees/`) or `claude --worktree`; do not edit files in the main directory. Clean up worktrees of merged branches as described in "Worktrees" in `AGENTS.md` (`task worktrees:prune`).
- Before committing, run the checks from the commands table; the `lefthook` hooks run on their own, do not bypass them.
