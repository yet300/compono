# Performance

Do not optimize on intuition. Establish: a representative workload; release-mode measurements; hardware and compiler metadata; latency distribution (not only averages); throughput; peak memory; allocation counts; bytes processed; cache behavior when relevant; regression thresholds.

Always measure with `--release`. Dev profiles are not production.

## Algorithms and layout first

1. Asymptotic behavior
2. Bounded candidate sets
3. Avoid unnecessary work
4. Contiguous data layout
5. Reduce bytes read and written
6. Avoid allocation and copies
7. Cache locality
8. Branch predictability
9. Vectorization or arch-specific kernels only after the above

Fewer arithmetic ops is not a speedup if memory traffic, cache misses, or synchronization dominate.

## Clones and copies

Find with review and profiling: clones inside loops; large structs passed by value; repeated string/byte conversions; serialize-then-immediately-deserialize; intermediate `collect`; copies across abstraction boundaries.

Do not remove a clone if that makes ownership unsound or materially harms clarity. Fix the ownership model rather than introducing fragile references.

Clippy: `redundant_clone`, `needless_collect`, `clone_on_copy`. `cargo clippy -- -D clippy::perf`.

## Reuse prepared state

For allocation-free steady state: validate and prepare once; precompute immutable tables; allocate permitted capacity at init; reuse worker-local scratch; reset lengths and cursors rather than reconstructing containers; keep repeated operations free of lazy init.

## Inspect generated code selectively

Assembly or LLVM IR for critical kernels, to verify: bounds-check elimination, vectorization, unexpected calls, hidden allocation, integer ops, branch structure, arch-specific instructions. Complements tests; does not replace them.

`#[inline]` only when a benchmark proves it. The compiler already inlines well.

Keep small `Copy` types on the stack. Avoid passing huge types by value. Heap-allocate recursive structures (`Box` around children). Large `const` arrays: do not materialize them on the stack then box; build a boxed slice without a giant stack temporary. Spill-capable small-vectors are forbidden on strict / allocation-free paths unless spilling is structurally impossible.

## Tooling

- [`cargo bench`](https://doc.rust-lang.org/cargo/commands/cargo-bench.html) for microbenchmarks. Treat a stable >5% win as interesting, not automatic merge.
- [`cargo flamegraph`](https://github.com/flamegraph-rs/flamegraph) (`cargo install flamegraph`). Always profile `--release`. Width is time on CPU; color is meaningless. On macOS, [samply](https://github.com/mstange/samply) is often a better DX.
- Benchmarks used as merge gates MUST have stable fixtures, known warmup, hardware metadata, noise-aware thresholds, allocation counters where relevant, separate correctness tests, and recorded baseline changes. Do not make fragile microbenchmark noise a correctness gate.

Keep a dedicated optimized test lane with `debug-assertions = true` ([errors.md](errors.md), [quality.md](quality.md)). Normal release benchmarks reflect production settings.
