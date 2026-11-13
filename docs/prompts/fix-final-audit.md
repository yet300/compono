# Prompt: fix the findings of the final audit of Compono

Written in ASD-STE100 Simplified Technical English. Give this whole file to the
engineering agent as its task.

---

## 1. Your role

You are a software engineer. You repair defects in the Rust navigation library
Compono and in its native packages.

You do not only find problems. You change the code, you add tests, and you
update the documentation. A task is complete only when the code is correct and
all checks pass.

Each task in section 4 is a known defect. An audit found each one, and a second
engineer examined the code and confirmed it. Do not do a new review of the
codebase. Do the tasks.

## 2. Before you start

1. Read `AGENTS.md`. It gives the rules and the checks for this repository.
2. Read `ARCHITECTURE.md`. It describes the design.
3. Use these skills from `.agents/skills`, as `AGENTS.md` tells you:
   - `test-driven-development` for each task.
   - `rust-best-practices` for all Rust code.
   - `rust-proof-techniques` for tasks 3, 7 and 8.
   - `compono-native` for tasks 4 and 9.
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
   doc comments, `ARCHITECTURE.md`, `CHANGELOG.md` (section "Unreleased") and
   the skills in `skills/` and `.agents/skills`. After a skill change, use
   `scripts/sync-skills.sh`.

If an existing test asserts the old behaviour, change that test. Do not delete
it. Record the change in your report.

If a test passes before you change the code, the test is not correct. Change
the test until it fails for the correct reason.

The version is `1.0.0-rc.1`. `VERSIONING.md` permits changes to the public API
before `1.0.0`. Record each change to the public API in `CHANGELOG.md`.

## 4. Tasks

Do the tasks in this sequence. Tasks 1 to 4 are small. Tasks 5 to 7 change the
public API. Task 9 changes the generated bindings.

### Task 1: the list of woken local tasks has no limit

- **Location:** `crates/compono/src/concurrency/local.rs`, `Woken::mark` and
  `TaskWaker`.
- **Defect:** Each wake-up adds `(node, id)` to the list. The code does not
  examine whether the task is already in the list. A future whose waker another
  thread calls many times between two pumps makes the list grow without limit.
  Each call also wakes the host.
- **Required result:** A task is in the list one time or not at all. The length
  of the list is not more than the number of live local tasks. The host is woken
  one time for each task that goes into the list.
- **Design:** Give each `TaskWaker` an `AtomicBool` that records "in the list".
  Set it with `swap(true)`. Add the task to the list only when the old value was
  `false`. Clear the flag before the pump polls the task.
- **Test:** Wake one task 10 000 times between two pumps. Make sure that the list
  holds one item, that the host is woken one time, and that the task is polled
  one time.
- **Concurrency:** This task changes `crates/compono/src/concurrency`. Extend the
  loom model in `crates/compono/tests/loom.rs`: a wake-up that occurs while the
  pump polls the task must not be lost.

### Task 2: a `NativeTree` dropped on a different thread may never be freed

- **Location:** `crates/compono-ffi/src/native_tree.rs`, `impl Drop for
  NativeTree` (line 320).
- **Defect:** When the last reference is dropped on a thread that is not the
  owner thread, `Drop` only adds the tree id to the `retired` list. The owner
  thread removes it at its next call. If no call comes, the tree, its `Platform`
  and the Swift or Kotlin listener stay in memory.
- **Required result:** The owner thread gets a request to collect the tree.
- **Design:** Keep an `Arc<dyn Platform>` in `NativeTree`. `Platform` is already
  `Send + Sync`. After `Drop` adds the id to `retired`, call
  `platform.schedule_pump()`. Make sure that a pump on a thread with no live tree
  still collects the retired trees.
- **Test:** Drop a `NativeTree` on a second thread. Make sure that
  `schedule_pump` is called, and that the record is gone after the owner thread
  does the call that `schedule_pump` asked for.

### Task 3: a `perform` task that panics leaves its guard

- **Location:** `crates/compono/src/concurrency/task.rs`, `Cx::perform`
  (line 96).
- **Defect:** If the future panics, it does not send its result. Its
  `TaskGuard` stays in `slot.tasks` until the component is removed. A
  long-lived component that starts tasks again and again collects these guards.
  This occurs only in builds that unwind. Release builds of the showcase use
  `panic = "abort"`.
- **Required result:** A task that panics removes its guard at the next pump.
  The component gets no message from that task.
- **Design:** Use `AssertUnwindSafe(future).catch_unwind()` from
  `futures`-style code only if it adds no dependency. If it needs a new
  dependency, write a small wrapper future that uses
  `std::panic::catch_unwind` around each poll. On a panic, send an envelope with
  the task id and no payload. `pump` then removes the guard and delivers
  nothing.
- **Test:** Start a task whose future panics, on `compono_test::VirtualTime`.
  Make sure that after `settle`, the component holds no guard for that task,
  and that a later task of the same component still works.

### Task 4: the showcase FFI checks the kind of a child only in debug builds

- **Location:** `native/showcase/ffi/src/component.rs`, `same_kind` (line 81),
  and the 11 typed accessors that use it.
- **Defect:** In a release build, an accessor given a child of the wrong kind
  returns a component object that answers nothing.
- **Required result:** Each of the 11 accessors returns `Option<...>`. It returns
  `None` for a child of a different kind, in debug and in release builds.
- **Native code:** Update every caller in `native/showcase/ios` and
  `native/showcase/android`. Use the `compono-native` skill to make the bindings
  again with `boltffi pack`. If `boltffi` is not installed, install version
  0.32.1 as the skill tells you.
- **Test:** In the showcase FFI tests, give each accessor a child of a different
  kind. Make sure that it returns `None`.

### Task 5: `cx.destroy` and `cx.set_child_phase` do nothing without an error

- **Location:** `crates/compono/src/component/context.rs`, `Cx::destroy` and
  `Cx::set_child_phase` (near line 228).
- **Defect:** `cx.send` returns `Err(NotOwnChild)` for a component that is not
  a live direct child. `cx.destroy` and `cx.set_child_phase` do nothing and
  return `()` in the same case.
- **Required result:** Both return `Result<(), NotOwnChild>` and are
  `#[must_use]`, as `cx.send` is.
- **Call sites:** Update each caller in the workspace, in
  `native/showcase/ffi` and in `benchmarks`. If a caller cannot get the error,
  write a comment that tells why.
- **Test:** For each method: a call for a live child succeeds, and a call for a
  grandchild and for a removed child returns `Err(NotOwnChild)`.

### Task 6: a router or a child that saves nothing gives no signal

- **Location:** `crates/compono/src/component/context.rs` (`Cx::router`,
  `Cx::named_routers`), `crates/compono/src/tree/routers.rs` (`name_router`)
  and `crates/compono/src/tree/mod.rs` (the repeated child name).
- **Defect:** In these cases a router or a child saves nothing, and nothing tells
  the application:
  - a component made more than 32 767 numbered routers;
  - a router name is used two times in one component;
  - a child name is used two times among live siblings;
  - `reusing_routers` was given an id it does not accept.
- **Required result:** The host gets a diagnostic for each case. Do not make the
  positional APIs deprecated. Do not panic.
- **Design:** Add a provided method to `Host`:
  `fn diagnostic(&mut self, diagnostic: &Diagnostic<'_>) {}`. Its default does
  nothing, so the change is additive. `Diagnostic` is a `#[non_exhaustive]`
  enum with one variant for each case above. Each variant holds the `NodeId`
  of the owner. Export it from the crate root. Make `compono_test`'s recording
  host record diagnostics, so tests can examine them.
- **Test:** One test for each case. Each test makes sure that the host received
  the correct diagnostic one time.
- **Documentation:** In the app skills `skills/compono-components/SKILL.md` and
  `skills/compono-navigation/SKILL.md`, tell applications to log diagnostics in
  debug builds.

### Task 7: saved state has no type identity

- **Location:** `crates/compono/src/persist.rs`: `Persist`, `Cx::restored`
  (line 466) and `Tree::with_saved` (line 155).
- **Defect:** A saved entry is found by a path of numbers only. `restored()`
  decodes its bytes with `postcard::from_bytes`. postcard does not record types,
  and `from_bytes` does not reject bytes left after the value. Thus bytes of a
  different component can decode without an error, and the component gets
  incorrect state. Also, the only version is one schema number for the whole
  tree.
- **Required result:**
  1. Each saved entry records a tag: a stable hash of the component kind and
     a version number.
  2. `restored()` gives `None` when the tag does not agree with the component.
  3. `restored()` gives `None` when bytes are left after the value. Use
     `postcard::take_from_bytes` and examine the remainder.
  4. A component can read state of an older version of itself.
- **Design:**
  - Add to `Persist`: `const KIND: &'static str = "";` and
    `const VERSION: u32 = 0;`. An empty `KIND` means "no kind check", so that
    existing implementations compile.
  - Add `fn migrate(version: u32, bytes: &[u8]) -> Option<Self::Saved>` with a
    default that returns `None`. `restored()` calls it when `KIND` agrees and
    `VERSION` does not.
  - Use FNV-1a for the kind hash, as `named_key` in `tree/mod.rs` does.
  - The saved envelope changes. Bytes in the old format give
    `RestoreOutcome::Corrupt` or `RestoreOutcome::SchemaChanged`, and the
    application starts clean. Record this in `CHANGELOG.md`.
  - Set `KIND` on every `Persist` type in `examples/showcase`, and raise
    `showcase::saved::SCHEMA` and `SCHEMA` in `native/showcase/ffi/src/lib.rs`.
- **Tests:**
  - Bytes saved by a component of kind A are not given to a component of
    kind B at the same path.
  - Bytes with extra bytes at the end are refused.
  - A component of version 2 reads version 1 bytes through `migrate`.
  - A component with no `KIND` restores as before.
- **Allocations:** Make sure that `crates/compono/tests/allocations.rs` still
  passes. If the tag adds an allocation, record it and the reason.

### Task 8: `Option<NodeId>` uses 12 bytes

- **Location:** `crates/compono/src/component/handle.rs`, `NodeId`, and
  `crates/compono/src/tree/node_slot.rs`.
- **Defect:** `NodeId` holds `index: u32` and `generation: u32`. No value is free
  for `None`, so `Option<NodeId>` is 12 bytes. `NodeSlot` has nine such fields.
- **Required result:** `Option<NodeId>` is 8 bytes. `NodeId::to_bits` and
  `NodeId::from_bits` give the same values as before, so addresses on the native
  side do not change.
- **Design:** Store the generation as `NonZeroU32`, with an offset of one.
- **Test:** Add compile-time assertions for `size_of::<Option<NodeId>>() == 8`
  and for `size_of::<NodeSlot>()` at its new size. Add a test that `to_bits` and
  `from_bits` give the same values as before, for the first generation, a middle
  generation and the last generation that is issued.

### Task 9: the work limit permits too many polls of local tasks

- **Location:** `crates/compono/src/tree/execution.rs` (`DEFAULT_WORK_LIMIT`,
  line 13) and `crates/compono/src/concurrency/local.rs` (`poll_local`).
- **Defect:** One limit of 1 048 576 applies to messages and to polls of
  `perform_local` futures. A message is cheap. A poll runs user code and can take
  a long time. Thus one pump can hold the UI thread for many frames.
- **Required result:** Polls have their own limit, with a default of 1 024.
  `Tree::set_poll_limit(limit)` changes it. A limit of 0 is treated as 1.
  The message limit does not change.
- **Out of scope:** A time-based limit. Do not add it.
- **Test:** With a poll limit of 3, start 10 local tasks that are ready. Make sure
  that one pump polls 3 of them and wakes the host, and that later pumps poll
  each task one time and deliver all 10 results.

## 5. Rules

- Library code has no `unwrap`, `expect`, `panic!`, indexing or `unsafe`. The
  lints enforce this.
- Do not add a dependency.
- Do not change files that a task does not need.
- Keep the documentation in English. It describes only the current state.
- Put a time limit on each long command, for example
  `perl -e 'alarm 900; exec @ARGV' cargo test --workspace`. If a command stops
  at the limit, record it. Do not wait.
- If a task needs a decision that this prompt does not give, select the option
  that changes the public API less. Record the decision and the reason in your
  report.
- If you cannot complete a task, stop that task, record why, and continue with
  the next task.

## 6. Checks before you report

Use all commands in the list "Before finishing a change" in `AGENTS.md`. These
include `cargo test -p compono --no-default-features`.

Also use these commands, because the tasks change these parts:

- Tasks 1, 3 and 9 change `crates/compono/src/concurrency`. Use the loom command
  from `AGENTS.md`.
- Tasks 7 and 8 change the tree bookkeeping. Use the three examples
  `navigation_cost`, `set_scaling` and `router_scaling` three times each. Make
  sure that the numbers do not increase as the sizes increase.
- `cargo test -p compono --release --test allocations`.
- Tasks 2, 4, 5 and 7 change the native side. Use the commands in the
  `compono-native` skill: `boltffi pack` for Apple and Android, clippy and
  tests of `native/showcase/ffi`, `swift test` in `native/apple/Compono`, the
  Gradle unit tests and `:app:assembleDebug`, and an `xcodebuild` build of
  `native/showcase/ios`.
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
