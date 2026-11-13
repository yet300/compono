# Prompt: fix the open review gaps in Compono

Written in ASD-STE100 Simplified Technical English. Give this whole file to the
engineering agent as its task.

---

## 1. Your role

You are a software engineer. You repair defects in the Rust navigation library
Compono.

You do not only find problems. You change the code, you add tests, and you
update the documentation. A task is complete only when the code is correct and
all checks pass.

Each task in section 4 is a known defect. Earlier reviews found and examined
each one. Do not do a new review of the codebase. Do the tasks.

## 2. Before you start

1. Read `AGENTS.md`. It gives the rules and the checks for this repository.
2. Read `ARCHITECTURE.md`. It describes the design.
3. Use these skills from `.agents/skills`, as `AGENTS.md` tells you:
   - `test-driven-development` for each task.
   - `rust-best-practices` for all Rust code.
   - `compono-verify` before you report.
   - `writing-for-agents` if you change a skill, `AGENTS.md` or `CLAUDE.md`.
4. The folder is not a git repository. Do not commit. Do not push.

## 3. How to do each task

Do these steps for each task, in this sequence:

1. Read the code at the given location. Make sure that the defect exists.
2. Write a test that shows the defect.
3. Start the test. Make sure that it fails, and that it fails for the correct
   reason. A compile error because of a missing function is not sufficient.
   Add a stub first, and then make the test fail on behaviour.
4. Change the code. Make the smallest correct change.
5. Start the test again. Make sure that it passes.
6. Start all tests of the crate. Make sure that no other test fails.
7. Update the documentation that describes the changed behaviour:
   doc comments, `ARCHITECTURE.md`, `CHANGELOG.md` and the skills in `skills/`
   and `.agents/skills`. After a skill change, use
   `scripts/sync-skills.sh`.

If an existing test asserts the old behaviour, change that test. Do not delete
it. Record the change in your report.

If a test passes before you change the code, the test is not correct. Change
the test until it fails for the correct reason.

## 4. Tasks

Do the tasks in this sequence. Tasks 1 to 3 are small. Task 4 is large.

### Task 1: `reusing_routers` accepts router ids of other routers

- **Location:** `crates/compono/src/component/context.rs`, `Cx::reusing_routers`
  (line 316) and `Cx::router` (line 330).
- **Defect:** The documentation says that the method accepts only ids that this
  component made. The code accepts every id with a number below the counter of
  the component. The code cannot know which component made an id. Thus these
  two errors are possible, and nothing reports them:
  - An id from a different component.
  - An id of a router of this component that is still alive. Two live routers
    then save their state under one id.
- **Required result:** `reusing_routers` gives an old id only to a router that
  replaces a router that is not alive. Write a test for each of the two errors
  above.
- **Constraint:** Do not change the public signature of `reusing_routers` or of
  `RouterId`. If the code cannot detect an error, make the documentation state
  the exact rule. Then record why in your report.

### Task 2: a rebuilt `Stack` loses restored state when the old stack is destroyed

- **Location:** `crates/compono/src/router/stack.rs`, `Stack::discard` (line 358),
  and the same pattern in the other routers.
- **Defect:** An owner builds a new stack inside
  `cx.reusing_routers(old.router_ids(), ...)` with
  `Stack::restore_or(Some(old.snapshot()), ...)`. Then the owner calls
  `old.destroy(cx)`. The old stack calls `cx.forget_entry` for each parked entry
  that it did not rebuild yet. The new stack has the same router id and the same
  entry keys. Thus the saved state of those entries is deleted, and the new stack
  cannot use it.
- **When it occurs:** Only for entries that the app restored at start and did not
  show yet.
- **Required result:** After this sequence, a parked entry of the new stack gets
  its saved state when the user shows it. Write a test that does a save, a
  restart, the rebuild and then shows the parked entry.
- **Constraint:** Saved state of a router that is destroyed and not replaced must
  still be removed before the next save, or at the next start. Keep the test that
  proves this, or add one.

### Task 3: the loop in `Tree::finish` has no limit

- **Location:** `crates/compono/src/tree/execution.rs`, `Tree::finish` (line 34)
  and `Tree::bury` (line 76).
- **Defect:** `finish` repeats two steps while dropped `Scope` values are waiting:
  it removes their components (`bury`), and then it delivers queued work
  (`drain`). The work limit applies to each `drain`, but not to the loop. Code
  that drops a new router each time a component is removed can keep the loop
  going without end.
- **Required result:** One operation does not do more work than the work limit,
  in all steps together. Count each removed component as one unit of work. When
  the limit stops the loop, keep the rest for the next operation and wake the
  host, as `drain` does now.
- **Test:** Write a test with a component that drops a new router in each
  `phase_changed`. Make sure that `tree.send` returns, and that `pump` continues
  the work later.

### Task 4: routers have no names in saved state

- **Defect:** A router gets its `RouterId` from a counter, in the sequence in
  which the owner constructs its routers. Children can have names
  (`Cx::child_keyed`, `Cx::persistent_child_keyed`). Routers cannot. If a later
  version moves one router constructor, or constructs a router only when a
  feature flag is set, a router gets the saved state of a different router.
- **Required result:** An owner can give a router a name. A named router finds
  its saved state by its name, independent of the sequence of construction.
- **Design rules:**
  1. Make one API that works for all built-in routers and for custom routers. Do
     not add a named constructor to each router. One possible form is
     `cx.named_routers("tabs", |cx| Stack::new(...))`, as `reusing_routers` does
     for ids.
  2. A name changes the saved key only. It does not change the behaviour of a
     router.
  3. Use the same stable hash as `named_key` in `crates/compono/src/tree/mod.rs`
     (FNV-1a).
  4. The saved keys of named routers must not be equal to the keys of numbered
     routers, of named children or of numbered children. Read the layout in
     `crates/compono/src/router/scope.rs` and in `tree/mod.rs` first. If the
     layout has no free space, change it, and increase nothing else. Record the
     new layout in a comment in `scope.rs` and in `ARCHITECTURE.md`.
  5. If a live router of the same owner already has the name, the second router
     must save nothing. Named children work this way now.
  6. Routers without a name must work as before. Saved state of an unnamed router
     from before your change must restore as before.
- **Tests:** Write tests for:
  - A named router keeps its state when a later version constructs a different
    router before it.
  - A repeated name does not share state.
  - A named router and a numbered router of one owner do not share state.
  - Unnamed routers save and restore as before.
- **Documentation:** Update `ARCHITECTURE.md`, `CHANGELOG.md`, the rule for new
  routers in `AGENTS.md`, `skills/compono-navigation/SKILL.md`,
  `skills/compono-navigation/references/custom-router.md` and
  `.agents/skills/compono-component/SKILL.md`. In the app skills, recommend names
  for routers with saved state.

### Task 5: the custom router in the showcase has no `destroy`

- **Location:** `examples/showcase/src/feature/gallery/filmstrip.rs`.
- **Defect:** `AGENTS.md` tells each router to offer `destroy(self, cx)`.
  `Filmstrip` does not. It is the example that the docs show for custom routers.
- **Required result:** Add `destroy(self, cx)`. It removes the components of all
  frames at once. Add a test in `examples/showcase/tests`.

## 5. Rules

- Library code has no `unwrap`, `expect`, `panic!`, indexing or `unsafe`. The
  lints enforce this.
- A change to a public item follows `VERSIONING.md`. Add a line to `CHANGELOG.md`
  for each change.
- Keep the documentation in English. It describes only the current state.
- Do not add a dependency.
- Do not change files that a task does not need.
- Put a time limit on each long command, for example
  `perl -e 'alarm 900; exec @ARGV' cargo test --workspace`. If a command stops
  at the limit, record it. Do not wait.
- If a task needs a decision that this prompt does not give, select the option
  that changes the public API less. Record the decision and the reason in your
  report.

## 6. Checks before you report

Use all commands in the list "Before finishing a change" in `AGENTS.md`. These
include `cargo test -p compono --no-default-features`.

Also use these commands, because the tasks change these parts:

- Tasks 2, 3 and 4 change the tree bookkeeping. Use the three examples
  `navigation_cost`, `set_scaling` and `router_scaling`. Make sure that the
  numbers do not increase as the sizes increase.
- Task 3 can change the work queue. If you change a file in
  `crates/compono/src/concurrency`, use the loom command from `AGENTS.md`.
- `scripts/sync-skills.sh --check`.

A task is complete only when all of these checks pass.

## 7. Report

Write the report in Markdown with these sections, in this sequence:

1. **Summary.** Three to six sentences. Tell which tasks are complete.
2. **One section for each task.** In each section, give:
   - the cause of the defect, with file path and line number;
   - the change that you made, with file paths;
   - the tests that you added, and the failure that each test gave before the
     change;
   - existing tests that you changed, and the reason;
   - the decisions that you made, and the reason for each.
3. **Checks.** Each command and its result. Give the number of tests that
   passed and failed.
4. **Not done.** Each item that you did not complete, and the reason.

Do not write that a task is complete if a check did not pass.
