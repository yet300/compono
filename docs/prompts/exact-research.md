# Prompt: research how Compono can work with Expo Exact

Written in ASD-STE100 Simplified Technical English. Give this whole file to the
research agent as its task.

---

## 1. Your role

You are a research engineer. You examine two codebases:

- **Compono**, a Rust library for components, navigation and saved state. It is
  in this repository.
- **Exact**, a cross-platform application runtime by Expo:
  <https://github.com/expo/exact>.

Your task is to find out if Compono can drive a user interface that Exact
draws, and how. You write a report. The owner uses the report to decide if an
engineer adds an Exact integration to Compono, and in which form.

You do not change files in this repository. You do not write the integration.
You can write small experiments outside the repository to prove a claim.

## 2. What is known before you start

This is a summary of the Exact README. Make sure that each point is true in the
code. Some points can be old, because Exact changes daily.

- Exact describes its UI in **Contract**, a declarative language. Contract
  compiles to a **plan** (tables and bytecode). A virtual machine, the
  **runner**, executes the plan.
- Logic and data come from **data sources** written in TypeScript (on Hermes,
  or the browser engine on the web) or in Rust (native, a shared library, or
  Wasm). Contract itself does not fetch data or run arbitrary code.
- A Contract component owns its `state`. Only `action`s change state.
  `resource` reads data and `mutation` sends changes.
- Hosts: web (DOM and CSS), macOS (AppKit), iOS (UIKit), Linux (GPU or CPU).
  Windows is partial. Android is early.
- Apple hosts are a static library with a C ABI. ExactKit embeds Exact in an
  existing Swift app.
- Exact has its own router (`routes`), with a retained stack for each tab.
- Exact is pre-1.0, with no API stability. The licence is MIT. The design
  records (`expo/exact-llp`) are private.

Compono already has these UI integrations. Use them as the model for what an
integration needs:

| Integration | Location | How the UI reaches the components |
|---|---|---|
| egui | `crates/compono-egui`, `examples/showcase-egui` | Views read `&Tree`, and an `Outbox` queues messages |
| iced | `crates/compono-iced`, `examples/showcase-iced` | Views return elements that carry an `Action` |
| GPUI | `crates/compono-gpui`, `examples/showcase-gpui` | A GPUI view owns the tree |
| Web | `crates/compono-web`, `examples/showcase-web` | Browser history, page lifecycle, session storage |
| SwiftUI and Compose | `crates/compono-ffi`, `native/` | A thread-bound tree behind FFI, one exported class per component |

## 3. Before you start

1. Read `ARCHITECTURE.md` in this repository, especially the sections "UI
   adapters", "Platform contract" and "Language boundary". They say what an
   adapter must do: wake the UI on a `Commit`, read components through `&Tree`,
   send messages, follow the phase of the window, and route Back.
2. Read the skills in `skills/`: `compono-components`, `compono-navigation`,
   `compono-ui` and `compono-native-apps`.
3. Clone Exact into a folder outside this repository. Record the commit that
   you examine. Every statement about Exact in your report is about that
   commit.
4. Read these Exact files first:
   - `README.md`
   - `docs/start-here.md`, `docs/contract-for-agents.md`,
     `docs/contract-grammar.md`, `docs/reference.md`, `docs/recipes/`
   - `rules/RULES.md`, `rules/DEFERRED.md`
   - the folders `kernel/`, `runner/`, `route/`, `data/`, `logic/`, `host/`,
     `js/`, `plan/`
   - one example app in `apps/` that uses a Rust data source, if one exists.

## 4. Questions that you must answer

Give each answer its identifier. For each answer, give the Exact file path and
line number, or the document, that proves it. If you cannot find proof, write
"Not found" and tell where you looked.

### Exact's model

- **E1.** How does a Rust data source connect to Exact? Give the trait, the
  functions or the ABI, and an example. Which crates must a Rust data source
  use?
- **E2.** How does data go from a data source to the UI? Is it a pull
  (`resource` asks) or a push (the data source tells the UI that data changed)?
  Can a data source push a change at any time, for example after a timer or a
  network reply?
- **E3.** How does a user event go from the UI to a data source? Give the path
  of a `mutation` or an `action` that calls Rust.
- **E4.** On which thread does a Rust data source run, on each host? Can it
  hold state between calls? Can it hold state for the whole life of the app?
- **E5.** How does a Rust data source build on each host: native, shared
  library, Wasm? What toolchain does each need?
- **E6.** How does Exact's router work: routes, stacks, tabs, Back, deep links?
  Can a data source own navigation, or does Contract always own it?
- **E7.** Does Exact report lifecycle to the app: the app or a screen is
  visible, hidden, focused? How does a data source know?
- **E8.** Does Exact save and restore UI state or navigation across a restart?
  How?
- **E9.** Does Exact support a list with many rows that creates content only
  for visible rows? How does it tell the app which rows are visible?

### Fit with Compono

- **F1.** Compono components own state and navigation. Exact components also
  own state, and Exact has a router. Which of these integration models is
  possible? Give the proof and the costs of each.
  - **Model A, Compono as a data source.** The Compono tree runs in a Rust data
    source. Contract screens read component models through `resource` and send
    messages through `mutation`. Contract draws what Compono's routers show.
  - **Model B, Compono owns navigation, Exact draws.** Compono's routers decide
    the screen. Exact's router is not used, or it only follows Compono.
  - **Model C, Exact owns navigation, Compono holds logic only.** Exact's router
    decides the screen. Compono components hold the logic of each screen, with
    no Compono routers.
  - Any other model that you find.
- **F2.** For the model that you recommend, map each Compono concept to its
  Exact equivalent: `Tree`, `Component`, `Handle`, `Phase`, `Commit`, the six
  routers, `Cx::perform`, `Persist`, `tree.back()`, `ListView`.
- **F3.** Can Exact wake the UI when the Compono tree commits from a background
  task? If not, what must poll, and how often?
- **F4.** Can the integration reuse an existing Compono crate:
  `compono-ffi`, `compono-web`, `compono-tokio`? Or does it need a new crate,
  such as `compono-exact`?
- **F5.** Does Exact's Back on each host reach the data source, so that
  Compono's `tree.back()` and predictive Back can work?
- **F6.** What does the integration need from Compono that Compono does not
  have now? Give each missing item.
- **F7.** What does the integration need from Exact that Exact does not have
  now? Give each missing item, and tell if Exact's `rules/DEFERRED.md` or an
  open issue plans it.

### Practical questions

- **P1.** Build and run one Exact example on the web, if the tools are
  available. Record each command and its result. Do not install a tool
  globally. If a tool is missing, record which tool and continue.
- **P2.** Licences: Exact is MIT, Compono is Apache-2.0. Make sure that the
  licences permit the integration. Record any file in Exact with a different
  licence.
- **P3.** Stability: Exact has no API stability. Tell how an integration can
  stay working: pin a commit, test against a version, or keep the integration
  outside this repository.
- **P4.** Platforms: for each host (web, macOS, iOS, Linux, Windows, Android),
  tell if the recommended model works there now.

## 5. How to do the research

1. Read code, not only documents. A document can be older than the code.
2. Put a time limit on each long command, for example
   `perl -e 'alarm 900; exec @ARGV' cargo build`.
3. Write experiments only outside this repository. Record the path of each
   experiment and what it proved.
4. Do not change files in this repository.
5. Do not guess. If a question has no answer in the code or the documents,
   write "Not found".
6. Do not copy large parts of Exact's code into the report. Give the path and
   the line number, and a short quote when it is necessary.

## 6. Output format

Write the report in Markdown. Use these sections in this sequence.

```markdown
# Compono and Exact: research report

## Summary

<Five to eight sentences. Is an integration possible? Which model do you
recommend, and why? What is the largest risk?>

## Exact at commit <hash>

<A short description of Exact's architecture as it is in the code: Contract,
plan, runner, data sources, hosts, router.>

## Answers

### E1: <question in a few words>

<Answer, with proof.>

<... one section for each identifier from E1 to P4 ...>

## Integration models

| Model | Possible | What it costs | What it loses |
|---|---|---|---|

## Recommended design

<The recommended model. The mapping from F2. The crates and files that an
engineer adds, with one line for each. The data flow for one example: a tap
on a button in a Contract screen pushes a screen in a Compono `Stack`, and the
new screen appears.>

## Proof of concept plan

<The smallest working example that proves the design, in numbered steps. It
must show: one Contract screen that reads a Compono component, one message
from the UI to the component, one navigation change, and one change that a
background task makes.>

## Risks

| Risk | Probability | Effect | What reduces it |
|---|---|---|---|

## Questions for the owner

<Decisions that the report cannot make.>

## Sources

<Each Exact file and document that you used, with the commit.>
```

## 7. Rules for the report

- Answer every question from E1 to P4.
- Keep facts and opinions apart. Mark each recommendation as a recommendation.
- Give a statement about Exact only with proof from the commit that you
  examined.
- If Exact cannot support an integration now, say so clearly. A clear "not
  now" is a correct result.
