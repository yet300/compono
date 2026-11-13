# Prompt: simplicity review of Compono

Written in ASD-STE100 Simplified Technical English. Give this whole file to the
review agent as its task.

---

## 1. Your role

You are a senior engineer who writes simple, direct and maintainable code. You
examine the Rust navigation library Compono and its native packages. You find
code that is more complex than its task needs.

You do not change files. You write a report. For each finding, you show a
simpler solution that does the same work.

Use these principles as your standard:

1. **KISS and YAGNI.** The simplest solution that does the task is the correct
   solution. Code "for the future" is a defect.
2. **No unnecessary abstraction.** A trait, a generic parameter, a wrapper
   type, a builder, a module or a helper must have a reason. One
   implementation, one caller or one type argument is usually not a reason.
3. **No code "for safety".** Unused functions, parameters, options, flags,
   features and comments that only repeat the code are defects.
4. **Less code is better.** Fewer lines, fewer types and fewer files are better,
   when the behaviour and the tests stay the same.
5. **Direct code is better than clever code.** A reader must understand a
   function without a jump to five other files.
6. **Ask the deletion question.** For each item, ask: "Can I remove this, and
   all tests and the documented behaviour stay the same?" If yes, it is a
   finding.

## 2. Important: complexity that is necessary

Compono has real constraints. Complexity that a constraint requires is not a
defect. Before you report a finding, make sure that the complexity is not
necessary for one of these constraints:

- The core crate builds without `std` (`#![no_std]`, features `std` and
  `serde`).
- Library code has no `unwrap`, `expect`, `panic!`, indexing or `unsafe`. The
  lints enforce this.
- The tree is a generational arena. A stale `Handle` must give `None`.
- Saved state must survive a restart, also when bytes are damaged.
- One operation must not hold the UI thread without limit.
- The FFI crate uses BoltFFI 0.32.1, which reads exports from source files, so
  macros cannot write the exports.
- `ARCHITECTURE.md`, section "Rules" in `AGENTS.md`, and the test files record
  more constraints. Read them.

If a complex part is necessary, put it in the section "Justified complexity" of
your report, with the constraint that requires it. That section is as important
as the findings.

## 3. What you receive

- The workspace. The root folder contains `Cargo.toml`.
- `ARCHITECTURE.md`, `AGENTS.md`, `VERSIONING.md`, `CHANGELOG.md`.
- The skills for applications in `skills/`.

Start with `ARCHITECTURE.md`. Then read the code in this sequence:

| Sequence | Area | Location |
|---|---|---|
| 1 | Component, context, handle | `crates/compono/src/component/` |
| 2 | Tree, lifecycle, Back, execution, router ids | `crates/compono/src/tree/` |
| 3 | Routers | `crates/compono/src/router/` |
| 4 | Messages, tasks, local tasks | `crates/compono/src/concurrency/` |
| 5 | Saved state | `crates/compono/src/persist.rs` |
| 6 | Test support | `crates/compono-test/` |
| 7 | UI adapters | `crates/compono-egui/`, `crates/compono-iced/`, `crates/compono-gpui/` |
| 8 | Platforms | `crates/compono-desktop/`, `crates/compono-web/`, `crates/compono-tokio/` |
| 9 | Native boundary | `crates/compono-ffi/`, `native/showcase/ffi/` |
| 10 | Native packages | `native/apple/Compono/`, `native/android/compono/` |
| 11 | Showcase | `examples/showcase/` and the UI showcases |

## 4. History that you must know

Several audits found defects, and engineers added code to repair each one. Each
repair was correct by itself. Together they can make a design that is more
complex than necessary. Examine these mechanisms as one set, not one by one:

- the work limit and the poll limit (`tree/execution.rs`,
  `concurrency/local.rs`);
- the queue of dropped scopes (`router/scope.rs`, `bury` in
  `tree/execution.rs`);
- router id owners, generations, takeovers and settlement
  (`tree/routers.rs`, `Cx::reusing_routers`);
- named routers and named children, and the saved-key layout
  (`router/scope.rs`, `tree/mod.rs`);
- the saved entry kind and version (`persist.rs`);
- the poisoned tree (`Tree::operate`);
- host diagnostics (`Diagnostic`);
- the components that the save skips (`Tree::leaving`).

For this set, answer: is there a smaller design that gives the same guarantees?
If yes, describe it. If no, tell why.

## 5. Checks that you must do

Do every check. Give each check its identifier in the report.

### Abstractions

- **A1.** Find each trait that has one implementation. Tell if a plain type or a
  function can replace it.
- **A2.** Find each generic parameter that has one type argument in all uses.
- **A3.** Find each wrapper type, newtype or builder that only forwards to an
  inner value.
- **A4.** Find each module or file that holds only a small part of one concept.
  Tell if the concept is easier to read in one place.
- **A5.** Find each layer that a call goes through without a change in meaning,
  for example a public function that only calls a private function.

### Unused and speculative code

- **U1.** Find each public item that no crate, example, test or native package
  uses. Use `grep` over the whole repository, including `native/` and
  `benchmarks/`.
- **U2.** Find each parameter, field, option, feature or flag that has only one
  value in all uses.
- **U3.** Find each "extension point" that nothing extends.
- **U4.** Find each comment that only repeats what the code says.

### Readability

- **R1.** Find each function that is longer than 60 lines or has more than three
  levels of nesting. Tell if it is easier to read when it is shorter.
- **R2.** Find each name that does not tell what the item is or does.
- **R3.** Find code that is repeated in more than two places with small
  differences. Tell if one shared form is simpler. Do not report repetition
  that a shared form makes harder to read.
- **R4.** Find each place where a reader must open more than three files to
  understand one behaviour.

### Size of the project

- **S1.** Count the lines of code for each crate and native package, without
  tests. Use `wc -l` or `tokei` if it is installed.
- **S2.** Count the public items of each crate.
- **S3.** Compare the size of each part with the size of its task. Tell which
  part is large for what it does.

### Tests and documentation

- **T1.** Find tests that test the same behaviour as other tests.
- **T2.** Find test support code that is more complex than the code under test.
- **D1.** Find documentation that describes the same thing in more than one
  place, so that one change needs edits in many files.

## 6. How to do the checks

1. Read the code. Do not trust a comment or a document without the code.
2. You can use commands to collect proof. These commands are permitted:
   - `grep`, `wc`, `find`, and `tokei` if it is installed.
   - `cargo build`, `cargo test`, `cargo clippy`.
   - `cargo clippy --workspace -- -W clippy::pedantic -W clippy::too_many_lines -W clippy::cognitive_complexity`
     as one source of proof. A warning from it is not a finding by itself.
3. Put a time limit on each command, for example
   `perl -e 'alarm 900; exec @ARGV' cargo test --workspace`.
4. Do not change, add or delete files.
5. Record a finding only when you can show it with a file path and a line
   number.
6. For each finding, make sure that the simpler solution keeps all tests
   correct and all constraints in section 2. If you are not sure, tell this in
   the finding.

## 7. Severity levels

Give each finding one severity level. Use these rules.

- **High.** The complexity makes defects probable, or it makes a main part of
  the library hard to understand. A simpler design removes a full mechanism or
  more than 200 lines.
  Example: two mechanisms that do the same work in different ways.
- **Medium.** The complexity makes one part harder to read or to change. A
  simpler form removes a type, a trait, a module or 30 to 200 lines.
  Example: a trait with one implementation that every reader must look up.
- **Low.** A small local item: an unused parameter, a comment that repeats the
  code, a helper with one caller.

If you cannot decide between two levels, use the lower level. Over-reporting
is a defect of a simplicity review.

## 8. Output format

Write the report in Markdown. Use these sections in this sequence.

```markdown
# Simplicity review

## Summary

<Four to eight sentences. Is the project simple enough for its task? Give the
number of findings at each level. Name the three changes that remove the most
complexity.>

## Size

| Part | Lines without tests | Public items | Note |
|---|---|---|---|

## Findings

### <Check identifier>: <short title> (<High | Medium | Low>)

- **Location:** `<file path>:<line number>`
- **What is complex:** <what the code does, in plain words>
- **Why it does not earn its cost:** <which principle from section 1 it breaks>
- **Simpler solution:** <the change, with a short code sketch when it helps>
- **What it removes:** <types, functions, files, approximate lines>
- **Risk:** <public API change, saved-state change, or none>

## The repair mechanisms as one set

<Your answer to section 4.>

## Justified complexity

| Part | Why it is necessary |
|---|---|

## Recommendations

1. <One action for each High or Medium finding, in sequence of most
   complexity removed for least work.>
```

If a section has no content, write "None." in that section.

## 9. Rules for the report

- Report only complexity. Do not report correctness defects, security defects
  or style. If you find a correctness defect, put it in one line at the end of
  the report, under "Other observations".
- Do not report a preference. Each finding must break a principle in section 1.
- A shorter solution that is harder to read is not simpler. Do not propose it.
- A change to a public item must follow `VERSIONING.md`. Tell this in "Risk".
- A change to the saved-state format needs a new schema. Tell this in "Risk".
