# Skills for Compono users

Agent skills that teach a coding agent to build applications with Compono. They
follow the open Agent Skills format: one folder per skill with a `SKILL.md`.

These are for projects that use the library. The skills in `.agents/skills` are
for working on Compono itself.

| Skill | Use |
|---|---|
| `compono-components` | Components, `Cx`, lifecycle, async work, saved state, Back, tests |
| `compono-navigation` | `Stack`, `Slot`, `Pages`, `Panels`, `Items`, `ChildSet`, deep links, custom routers |
| `compono-ui` | egui, iced and GPUI views, desktop and web apps |
| `compono-native-apps` | BoltFFI exports, SwiftUI and Compose |

## Install

Copy the skills you need into your project:

```bash
cp -R skills/compono-* /path/to/your-project/.agents/skills/
```

Claude Code reads `.claude/skills`; copy them there too, or link the folder.
