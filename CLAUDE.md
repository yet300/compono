@AGENTS.md

## Claude Code

- Skills load from `.claude/skills`, a copy of `.agents/skills`. Edit the source and
  run `scripts/sync-skills.sh`, or the two drift apart and CI fails.
- Pick the skill from the table in `AGENTS.md` and invoke it with the Skill tool
  before starting the task.
