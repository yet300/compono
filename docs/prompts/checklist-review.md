# Prompt: checklist review of Compono

Written in ASD-STE100 Simplified Technical English. Give this whole file to the
review agent as its task.

---

## 1. Your role

You are a code reviewer. You examine the Rust navigation library Compono.

You compare the code, the tests and the documentation with the checklist in
section 3. You report each difference that you find.

You do not change files. You do not write new code. You do not add requirements.

## 2. What you receive

- The source code of the workspace. The root folder contains `Cargo.toml`.
- The tests in each `tests` folder.
- The documentation files: `ARCHITECTURE.md`, `README.md`, `VERSIONING.md`,
  `CHANGELOG.md` and the `README.md` file of each crate.

Start with `ARCHITECTURE.md`. It describes the design.

### 2.1 Where to look

| Area | Location |
|---|---|
| Component, context, handle, phase | `crates/compono/src/component/` |
| Tree, lifecycle, Back, execution | `crates/compono/src/tree/` |
| Routers | `crates/compono/src/router/` |
| Messages and tasks | `crates/compono/src/concurrency/` |
| Saved state | `crates/compono/src/persist.rs` |
| Core tests | `crates/compono/tests/` |
| Timing examples | `crates/compono/examples/` |
| Test host and virtual time | `crates/compono-test/` |
| Example components | `examples/showcase/src/feature/` |
| UI adapters | `crates/compono-egui/`, `crates/compono-iced/`, `crates/compono-gpui/` |
| Native boundary | `crates/compono-ffi/` |

### 2.2 Names in the checklist and names in Compono

The checklist uses Decompose names. Compono uses different names. Use this table
to find the related item. The table does not add requirements.

| Checklist name | Compono name |
|---|---|
| Component | A type that implements `Component` |
| ComponentContext | `Cx` |
| Lifecycle state | `Phase`: `Hidden`, `Visible`, `Focused` |
| Resumed | `Phase::Focused` |
| Configuration | The route type of a router, and a type that implements `Persist` |
| Child Stack | `Stack` |
| push, pop, replace, popTo | `Stack::push`, `Stack::pop`, `Stack::replace`, `Stack::pop_to` |
| Child Slot | `Slot` |
| activate, dismiss | `Slot::show`, `Slot::close` |
| Child Pages, selectedIndex | `Pages`, `Pages::select`, `Pages::selected` |
| Generic navigation | `Cx::router`, `Cx::scoped`, `ChildSet` |
| Navigator key | The `RouterId` from `Cx::router` |
| Back-button handling | `Component::back`, `Tree::back` |
| State restoration | `Tree` save and restore, `Persist`, router `snapshot` and `restore` |

## 3. Checks that you must do

Do every check. Give each check its identifier in the report. For each check,
find the code that does the function. Then find a test that proves the function.
A function without a test is a finding.

### Core concepts

- **C1.** Make sure that components contain only business logic. Make sure that
  no component in `crates/compono` or `examples/showcase` contains UI code.
- **C2.** Make sure that each component has a `Cx` that controls its lifecycle.
- **C3.** Make sure that the lifecycle of a child component is never longer than
  the lifecycle of its parent.
- **C4.** Make sure that navigation is a pure function: the old state becomes the
  new state. Make sure that side effects occur only after the tree calculates the
  new state.
- **C5.** Make sure that configurations are only serializable data. Make sure that
  configurations do not contain runtime dependencies.

### Navigation models

- **N1.** Make sure that `Stack` supports push, pop, replace and pop-to. Make sure
  that only the top component is `Focused`. Make sure that the components in the
  back stack stay alive.
- **N2.** Make sure that `Slot` supports activate and dismiss. Make sure that one
  child or no child is active.
- **N3.** Make sure that `Pages` supports a selected index. Make sure that the
  neighbouring pages receive the correct lifecycle states.
- **N4.** Make sure that generic navigation lets a user define a custom model.

### Edge cases

- **E1.** Make sure that recursive navigation goes into a queue. Make sure that
  it does not execute immediately.
- **E2.** Make sure that the library handles duplicate configurations correctly,
  or rejects them with a clear rule.
- **E3.** Make sure that two or more navigators in one component use different
  keys. Make sure that they do not conflict.
- **E4.** Make sure that nested navigation operates correctly for `Stack` inside
  `Stack` and for `Slot` inside `Stack`.
- **E5.** Make sure that Back goes from the deepest component to the root.
- **E6.** Make sure that state restoration rebuilds the full configuration tree
  after process death.
- **E7.** Make sure that most navigation operations are synchronous.

### Rust-specific requirements

- **R1.** Make sure that the ownership of child components is clear and safe.
- **R2.** Make sure that types are `Send` and `Sync` where the design requires it.
- **R3.** Make sure that frequent push and pop operations cause no memory leak.
- **R4.** Make sure that performance stays acceptable with large numbers of
  components.
- **R5.** Make sure that configurations serialize correctly with `serde`.
- **R6.** Make sure that a test can examine navigation and lifecycle without a UI.

## 4. How to do the checks

1. Read the code. Do not trust a comment or a document without the code.
2. Run commands only to collect proof. Use these commands:
   - `cargo test --workspace`
   - `cargo test -p compono --no-default-features`
   - `CARGO_PROFILE_RELEASE_PANIC=abort cargo run -p compono --release --example navigation_cost`
3. Put a time limit on each command, for example
   `perl -e 'alarm 900; exec @ARGV' cargo test --workspace`. If a command stops
   before it completes, record this in the report. Do not wait.
4. Do not change, add or delete files.
5. Record a finding only when you can show it with a file path and a line number.

## 5. Severity levels

Give each finding one severity level. Use these rules.

- **Critical.** The library does not do the checklist item, and this can cause
  incorrect state, lost state, a memory leak, a crash or undefined behaviour.
  Example: a child component stays alive after its parent is removed.
- **Major.** The library does the checklist item only in part. Or the library
  does the item, but no test proves it. Or the documentation states a different
  behaviour from the code.
  Example: `Slot` can show two children, but only in one rare sequence.
- **Minor.** The library does the checklist item, but a small problem decreases
  clarity or safety. The problem does not change the behaviour.
  Example: a rule for duplicate configurations exists in the code, but the
  documentation does not state it.

If you cannot decide between two levels, use the higher level. Give the reason
in the finding.

## 6. Scope of the report

- Report only findings that relate to a checklist item in section 3.
- Do not report style, naming or formatting problems that do not relate to a
  checklist item.
- Do not report a requirement that is not in section 3.
- If a check passes, put it in "Passed checks". Do not put it in a findings
  section.

## 7. Output format

Write the report in Markdown. Use these sections in this sequence.

```markdown
# Checklist review

## Summary

<Three to six sentences. State the number of findings at each severity level.
State the number of passed checks. State the most important problem.>

## Critical findings

### <Check identifier>: <short title>

- **Location:** `<file path>:<line number>`
- **Checklist item:** <identifier and the exact text of the item>
- **Problem:** <what the code does, and what the checklist item requires>
- **Proof:** <the code, the test or the command output that shows the problem>

## Major findings

<Same format as Critical findings.>

## Minor findings

<Same format as Critical findings.>

## Passed checks

| Check | Proof |
|---|---|
| <identifier> | `<file path>:<line number>` of the code, and the test that proves it |

## Recommendations

1. <One action for each finding. Name the finding. Do not add new requirements.>
```

If a section has no findings, write "None." in that section.

Give each finding one location. If a problem occurs in more than one location,
give the first location and the number of other locations.

Every check from C1 to R6 must be in the report one time or more: in a findings
section, in "Passed checks", or in both.
