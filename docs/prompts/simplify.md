# Prompt: make Compono simpler

Written in ASD-STE100 Simplified Technical English. Give this whole file to the
engineering agent as its task.

---

## 1. Your role

You are a software engineer. You make the Rust navigation library Compono
simpler. You remove code, types and steps that its task does not need. You keep
all behaviour that applications and tests use.

A simplicity review found each item in section 4. A second engineer examined the
code and confirmed it. Do not do a new review. Do the tasks.

Use these principles:

1. The simplest solution that does the task is the correct solution.
2. A trait, a type, a wrapper or a step must have a reason. If it has no
   reason, remove it.
3. Less code is better, when the behaviour and the tests stay the same.
4. Direct code is better than clever code. A shorter solution that is harder to
   read is not simpler.

## 2. Before you start

1. Read `AGENTS.md`. It gives the rules and the checks for this repository.
2. Read `ARCHITECTURE.md`. It describes the design.
3. Use these skills from `.agents/skills`, as `AGENTS.md` tells you:
   - `test-driven-development` for task 1.
   - `rust-best-practices` for all Rust code.
   - `compono-verify` before you report.
   - `writing-for-agents` if you change a skill, `AGENTS.md` or `CLAUDE.md`.
4. The folder is not a git repository. Before you start, make a copy of the
   whole folder outside the repository, so that you can compare and restore.
   Do not commit. Do not push.
5. Start all tests one time before you change code. Record the number of tests
   that pass. All must pass.

## 3. How to do each task

There are two types of task.

**A task that changes behaviour or adds API (task 1).** Do these steps:

1. Write a test for the new API or behaviour.
2. Make sure that the test fails for the correct reason. A compile error because
   of a missing function is not sufficient: add a stub first.
3. Change the code.
4. Make sure that the test passes and that all other tests pass.

**A task that removes code or changes its form only (tasks 2 to 9).** The
existing tests protect the behaviour. Do these steps:

1. Find the tests that examine the code that you change.
2. Change the code.
3. Make sure that all tests pass.
4. If a test examines only an item that you remove, remove that test too.
   Record each removed test and the reason in your report.
5. If a test examines a behaviour that stays, keep the test. Change only what
   the new form of the code needs.

After each task, update the documentation that describes the changed code:
doc comments, `ARCHITECTURE.md`, `VERSIONING.md`, `CHANGELOG.md` (section
"Unreleased"), and the skills in `skills/` and `.agents/skills`. After a skill
change, use `scripts/sync-skills.sh`.

These rules apply to all tasks:

- The saved-state format must not change. Bytes saved before your change must
  restore after it. The tests `unnamed_routers_save_the_same_bytes_as_before`
  and `unnamed_routers_restore_bytes_saved_before` in
  `crates/compono/tests/named_routers.rs` prove this. Do not change their bytes.
- The version is `1.0.0-rc.1`. `VERSIONING.md` permits changes to the public API
  before `1.0.0`. Record each change to the public API in `CHANGELOG.md`.

## 4. Tasks

Do the tasks in this sequence.

### Task 1: replace `reusing_routers` with a router replacement that consumes the old router

- **Location:** `crates/compono/src/tree/routers.rs` (`Takeover`,
  `take_over_router`, `settle_routers`), `crates/compono/src/component/context.rs`
  (`Cx::reusing_routers`, `Cx::router`), `crates/compono/src/router/scope.rs`
  (`RouterId::generation`, `with_generation`, the hand-written `PartialEq`),
  `crates/compono/src/tree/mod.rs` (the fields for generations and takeovers),
  `crates/compono/src/tree/execution.rs` (the call to `settle_routers`),
  `crates/compono/src/host.rs` (`Diagnostic::RefusedRouterId`).
- **Problem:** `reusing_routers` lets a new router take the ids of a router that
  can still be alive. Thus each id has a generation, the tree records each
  move, and the tree settles the moves at the end of an operation. A reader
  must open seven files to understand this. No application calls
  `reusing_routers`. Only tests call it.
- **Required result:** a method that consumes the old router:

  ```rust
  pub fn replace_router<R: Router<C>>(&mut self, old: R, build: impl FnOnce(&mut Self) -> R) -> R
  ```

  1. It destroys the old router first. While it does this, the saved state under
     the old router's ids stays. Calls to `forget_entry`, `forget_router` and
     `keep_router_entries` for those ids do nothing during this step.
  2. Then it builds the new router. The new router takes the old router's ids,
     in the same sequence. `Panels` has three ids.
  3. Two routers never hold one id, so the tree needs no generations, no
     takeover records and no settlement.
- **The `Router` trait:** add `pub trait Router<P: Component>` with
  `fn router_ids(&self) -> impl IntoIterator<Item = RouterId>` and
  `fn destroy(self, cx: &mut Cx<'_, P>)`. Implement it for the six built-in
  routers and for `Filmstrip` in the showcase. This trait records the rule for
  routers that `AGENTS.md` already gives. Keep the inherent methods only if a
  trait method cannot replace them.
- **Remove:** `Cx::reusing_routers` (or make it private, if `replace_router`
  needs it), `Takeover`, `take_over_router`, `settle_routers`,
  `router_entries`, the generation table, the `generation` field of `RouterId`
  and `with_generation`. `RouterId` then derives `PartialEq`. Keep a set of the
  names that a component has given out, because a repeated name must still
  save nothing. Remove `Diagnostic::RefusedRouterId` if no case can give it.
- **Tests:**
  - New: a `Stack`, a `Slot` and a `Panels` replaced with `replace_router` keep
    the saved state of their entries after a restart.
  - New: a `Stack` restored at start and replaced before its parked entries are
    shown keeps the saved state of those entries.
  - New: a router destroyed without a replacement leaves no saved state.
  - Change the tests in `rebuilt_router.rs`, `rebuilt_restored.rs`,
    `reused_router_ids.rs`, `named_routers.rs` and
    `compono-test/tests/diagnostics.rs` to use `replace_router`, where they
    examine a behaviour that stays.
  - Remove `pending_takeover.rs` and each test that examines only a state that
    `replace_router` makes impossible: a replaced router that is still alive,
    or an id from another router. Record each one.
- **Documentation:** `ARCHITECTURE.md` (section on router ids),
  `skills/compono-navigation/SKILL.md`,
  `skills/compono-navigation/references/custom-router.md`, the router rule in
  `AGENTS.md`, `.agents/skills/compono-component/SKILL.md`.

### Task 2: remove `Tree::back_progress` and `BackGesture`

- **Location:** `crates/compono/src/tree/back.rs:50`,
  `crates/compono/src/tree/mod.rs` (`BackGesture`).
- **Problem:** No adapter and no native package calls `back_progress`. Swift and
  Kotlin keep the progress of a gesture in the UI. `ARCHITECTURE.md:656` says
  so, and `ARCHITECTURE.md:330` says the opposite.
- **Required result:** Remove `back_progress`, `back_progress_now`, the
  `progress` field and the `BackGesture` type. Replace `Tree::gesture()` with
  `pub const fn gesture_target(&self) -> Option<NodeId>`. Update the caller in
  `crates/compono-ffi/src/native_tree.rs:279`.
- **Documentation:** Correct `ARCHITECTURE.md:330`, so that both places say that
  the progress stays in the UI.

### Task 3: remove the unused model types of `compono-ffi`

- **Location:** `crates/compono-ffi/src/model.rs` (`Project`, `NativeChild`,
  `RouterEntry`), `crates/compono-ffi/src/native_tree.rs` (`NativeTree::model`).
- **Problem:** The native showcase does not use these items. It has its own
  `ChildRef`. Only the test of `compono-ffi` and
  `benchmarks/ffi-boundary/scenario` use them. `VERSIONING.md` still promises
  that the fields of `RouterEntry` do not change.
- **Required result:** Remove the four items. Keep `phase_code`,
  `phase_from_code` and `NO_NODE`. Change the benchmark scenario to read the
  model with `NativeTree::read`. Remove the `RouterEntry` line from
  `VERSIONING.md` and the `Project` text from `ARCHITECTURE.md`.
- **Check:** Build `benchmarks/ffi-boundary` after the change.

### Task 4: one function for the four "forget restored state" loops

- **Location:** `crates/compono/src/persist.rs:301` (`forget_subtree`), `:318`
  (`forget_saved`), `:337` (`forget_router`), `:360` (`retain_router`).
- **Problem:** The four functions do the same scan of `restored` with different
  conditions.
- **Required result:** One private function with a path prefix and a
  condition on the key after the prefix. The four callers use it. Keep the scan
  start at the first key of the router when the router range is known, so that
  the scan does not become longer.

### Task 5: remove layers that only pass a call through

- **Location and change:**
  - `crates/compono/src/tree/mod.rs`: `Tree::parent` only calls `parent_of`.
    Keep one of them.
  - `crates/compono/src/tree/node_slot.rs:64`: `first_child_for_save` only
    returns a field. Use the field.
  - `crates/compono/src/concurrency/sender.rs:135`: `deliver` only calls `push`.
    Call `push`.
  - `crates/compono-web/src/history.rs:216`: `navigation_to` only calls
    `Navigation::toward`. Call it directly.
  - `native/showcase/ffi/src/component.rs:80`: `same_kind(a, b)` is `a == b`.
    Write `==`.
- **Rule:** Keep each public item. Remove only private hops.

### Task 6: put the saved-key layout in one module

- **Location:** `crates/compono/src/router/scope.rs:9-32` and
  `crates/compono/src/tree/mod.rs:11-19` and `:40` (`AUTO_KEY_BASE`,
  `NAMED_KEY_BASE`, `REPEATED_NAME_KEY`, `named_key`), and the overlap test at
  `tree/mod.rs:606`.
- **Required result:** All four sections of the 64-bit key, their constants,
  their constructors and the overlap test are in one module. The keys do not
  change.

### Task 7: one shared null host for the tests of `compono`

- **Location:** about 20 files in `crates/compono/tests/` define their own
  `NullHost`.
- **Problem:** `compono_test::NoHost` cannot be used: a dev-dependency on
  `compono-test` turns on the default features of `compono`, and
  `cargo test -p compono --no-default-features` then fails.
- **Required result:** `crates/compono/tests/common/mod.rs` holds `NullHost`.
  Each test file that needs it uses `mod common;`. Keep a local host where a
  test needs a host with different behaviour.

### Task 8: make `Tree::allocate` shorter

- **Location:** `crates/compono/src/tree/mod.rs` (`Tree::allocate`, 91 lines).
- **Required result:** Move the choice of the saved key to its own function,
  and the `NodeSlot` literal to `NodeSlot::new`. `allocate` keeps only the
  sequence of steps. The behaviour does not change.

### Task 9: state each fact once in the documentation

- **Problem:** The limit of 32 767 router ids is stated in six places, and the
  saved-key table in two.
- **Required result:** State each number and the table one time, in the rustdoc
  of the item that enforces it. Other places refer to that item. Do the same
  for each fact that tasks 1 to 8 changed.

## 5. Out of scope

Do not do these things. A review proposed them, and the owner decided to keep
the current code:

- Do not remove `Persist::VERSION` or `Persist::migrate`.
- Do not merge the poll limit with the work limit.
- Do not remove `ListView::overscan` or `Key::MouseBack`.
- Do not rename private items or test files.
- Do not merge or move tests that this prompt does not name.
- Do not change the `docs/` folder.

## 6. Rules

- Library code has no `unwrap`, `expect`, `panic!`, indexing or `unsafe`. The
  lints enforce this.
- Do not add a dependency.
- Do not change files that a task does not need.
- Keep the documentation in English. It describes only the current state.
- Put a time limit on each long command, for example
  `perl -e 'alarm 900; exec @ARGV' cargo test --workspace`. If a command stops
  at the limit, record it. Do not wait.
- If a task needs a decision that this prompt does not give, select the option
  that keeps the public API closer to the current API. Record the decision and
  the reason in your report.
- If you cannot complete a task, restore its files from your copy, record why,
  and continue with the next task.

## 7. Checks before you report

Use all commands in the list "Before finishing a change" in `AGENTS.md`. These
include `cargo test -p compono --no-default-features`.

Also use these commands:

- The loom command from `AGENTS.md`, because task 5 changes
  `crates/compono/src/concurrency`.
- The three examples `navigation_cost`, `set_scaling` and `router_scaling`,
  three times each, because task 1 changes the tree bookkeeping. Make sure that
  the numbers do not increase as the sizes increase.
- `cargo test -p compono --release --test allocations`.
- Tasks 2, 3 and 5 change the native side. Use the commands in the
  `compono-native` skill: `boltffi pack` for Apple and Android, clippy and
  tests of `native/showcase/ffi`, `swift test` in `native/apple/Compono`, the
  Gradle unit tests and `:app:assembleDebug`, and an `xcodebuild` build of
  `native/showcase/ios`.
- `cargo check` in `benchmarks/ffi-boundary` and in
  `benchmarks/decompose-comparison/rust`.
- `scripts/sync-skills.sh --check`.

Count the lines of code of `crates/compono/src` before and after, without
tests. Record both numbers.

A task is complete only when all of these checks pass.

## 8. Report

Write the report in Markdown with these sections, in this sequence:

1. **Summary.** Three to six sentences. Tell which tasks are complete, and how
   many lines the change removed.
2. **One section for each task.** In each section, give:
   - the change that you made, with file paths;
   - the items that you removed;
   - the tests that you added, and the failure that each gave before the change;
   - the tests that you removed or changed, and the reason for each;
   - the decisions that you made, and the reason for each.
3. **Size.** Lines of code of `crates/compono/src` before and after, and the
   number of public items before and after.
4. **Checks.** Each command and its result. Give the number of tests that
   passed and failed.
5. **Not done.** Each item that you did not complete, and the reason.

Do not write that a task is complete if a check did not pass.
