# Agent skills

Skills follow the open Agent Skills format: one folder per skill with a
`SKILL.md`. This folder is the source. `.claude/skills` is a copy for Claude Code,
refreshed by `scripts/sync-skills.sh`.

## Compono

| Skill | Use |
|---|---|
| `compono-verify` | Which checks a change needs, and how to run them without hangs |
| `compono-component` | Components, routers, saved state and showcase screens |
| `compono-native` | `compono-ffi`, BoltFFI, the Swift package, the Compose library, simulators |

## Vendored

Each keeps its upstream `LICENSE` (MIT). Local edits are listed.

| Skill | Source | Commit | Local edits |
|---|---|---|---|
| `rust-best-practices` | [massimodeluisa/rust-best-practices-skill](https://github.com/massimodeluisa/rust-best-practices-skill) | `0061c338` | `evals/` left out |
| `test-driven-development` | [obra/superpowers](https://github.com/obra/superpowers) | `22f0b463` | Cross-reference renamed to `writing-for-agents` |
| `systematic-debugging` | [obra/superpowers](https://github.com/obra/superpowers) | `22f0b463` | `superpowers:` prefixes dropped. The TypeScript example and `find-polluter.sh` left out, and their mentions replaced with Rust equivalents |
| `verification-before-completion` | [obra/superpowers](https://github.com/obra/superpowers) | `22f0b463` | None |
| `writing-for-agents` | [mattpocock/skills](https://github.com/mattpocock/skills) | `4ae2317d` | `agents/openai.yaml` left out |
| `rust-proof-techniques` | [leonardomso/rust-skills](https://github.com/leonardomso/rust-skills) | `fd2a861` | Four of its 265 rules, unchanged except for links to rules not copied. `SKILL.md` is written here |

Considered and not taken: the rest of `leonardomso/rust-skills`, whose advice on
`thiserror`, `expect` and extra collection crates contradicts `AGENTS.md`, and
`actionbook/rust-skills`, whose skills lean on web and Tokio, link to each other
throughout and run shell commands when loaded.

Where a vendored skill and `AGENTS.md` disagree, `AGENTS.md` wins. To update one,
copy the new upstream version, reapply the edits above, and run the sync script.
