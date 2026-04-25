# Perry Benchmarks

This is the canonical, single-page comparison of Perry against
production-quality runtimes — **node, bun, Rust, Go, C++, Swift,
Java, Python, Zig**. It pulls together every benchmark in this
repo, lists the exact compiler flags used per language, calls out
where Perry leads and where it doesn't, and links to the design
docs that explain *why* the numbers look the way they do.

The format is designed for skeptics. Every implementation, every
flag, every methodology decision is in this page — no tables hidden
behind blog posts, no cherry-picked subsets.

> **Hardware:** Apple M1 Max (10 cores: 8P + 2E), 64 GB RAM, macOS
> 26.4. Numbers from 2026-04-25 unless otherwise stated.
>
> **CPU pinning:** macOS `taskpolicy -t 0 -l 0` — sets throughput-tier 0
> + latency-tier 0, a scheduler HINT toward P-cores on Apple Silicon.
> This is **not** strict pinning; Apple does not expose unprivileged
> hard core affinity. (`taskpolicy -c user-interactive` does not exist;
> the `-c` clamp only accepts downgrade values utility/background/
> maintenance.) On Linux the runner uses `taskset -c 0` for strict
> pinning instead. The runner prints which strategy was applied at
> the top of each invocation.
>
> **Methodology:** RUNS=11 per cell (configurable via `$RUNS`). For
> each cell we collect every per-run wall-clock ms and report
> **median, p95, σ (population stddev), min, and max** — not
> "best-of-N". Headline tables show the median; full distributions
> are in [`json_polyglot/RESULTS.md`](json_polyglot/RESULTS.md) and
> [`polyglot/RESULTS.md`](polyglot/RESULTS.md). Time in milliseconds,
> RSS in MB (peak resident set size from `/usr/bin/time -l`, the worst
> peak observed across runs).
>
> **Warmup:** the bench programs themselves run 3 untimed warmup
> iterations before the timed loop, to avoid charging JIT-y runtimes
> (Perry's compiled binary, V8, JSC, JVM) for cold-start. Process
> startup is included in the timed window for non-JIT runtimes (Go,
> Rust, C++, Swift) since their startup is sub-millisecond.

---

## TL;DR

### JSON benchmarks — two workloads, both headline

10k records, ~1 MB blob, 50 iterations per run. Same data generator
across both. RUNS=11 per cell. Headline = median ms. Full per-cell
stats (median + p95 + σ + min + max) in
[`json_polyglot/RESULTS.md`](json_polyglot/RESULTS.md).

#### A. JSON validate-and-roundtrip
> Per iteration: `parse(blob)` → `stringify(parsed)` → discard.

The unmutated parse lets Perry's lazy tape (v0.5.204+) memcpy the
original blob bytes for stringify, which is why Perry's headline
number on this workload is so low — the lazy path can avoid
materializing the parse tree entirely. This is the honest "Perry
beats everyone" workload because it's a workload Perry's runtime is
specifically optimized for.

| Implementation | Profile | Median (ms) | p95 (ms) | σ | Min | Max | Peak RSS (MB) |
|---|---|---:|---:|---:|---:|---:|---:|
| **perry (gen-gc + lazy tape)** | optimized | **70** | 74 | 1.7 | 68 | 74 | 85 |
| rust serde_json (LTO+1cgu) | optimized | 187 | 318 | 40.6 | 184 | 318 | 13 |
| rust serde_json | idiomatic | 202 | 208 | 2.2 | 198 | 208 | 12 |
| bun | idiomatic | 260 | 270 | 5.0 | 255 | 270 | 84 |
| perry (mark-sweep, no lazy) | idiomatic | 366 | 393 | 11.8 | 359 | 393 | 102 |
| node --max-old=4096 | optimized | 395 | 761 | 123.4 | 381 | 761 | 182 |
| node | idiomatic | 396 | 486 | 28.8 | 379 | 486 | 182 |
| kotlin -server -Xmx512m | optimized | 457 | 490 | 12.8 | 451 | 490 | 426 |
| kotlin (kotlinx.serialization) | idiomatic | 484 | 495 | 7.1 | 469 | 495 | 608 |
| c++ -O3 -flto (nlohmann/json) | optimized | 788 | 807 | 6.5 | 780 | 807 | 25 |
| go -ldflags="-s -w" -trimpath | optimized | 823 | 885 | 18.7 | 812 | 885 | 22 |
| go (encoding/json) | idiomatic | 831 | 1123 | 92.9 | 811 | 1123 | 23 |
| c++ -O2 (nlohmann/json) | idiomatic | 872 | 1550 | 304.2 | 858 | 1550 | 28 |
| swift -O (Foundation) | idiomatic | 3747 | 5108 | 391.2 | 3713 | 5108 | 34 |
| swift -O -wmo (Foundation) | optimized | 3778 | 4395 | 178.1 | 3763 | 4395 | 35 |

#### B. JSON parse-and-iterate
> Per iteration: `parse(blob)` → sum every record's `nested.x`
> (touches every element) → `stringify(parsed)` → discard.

The full-tree iteration FORCES Perry's lazy tape to materialize, so
this is the honest comparison for workloads that touch JSON content.
Perry doesn't lead here — when you can't avoid the work, the lazy
tape pays its overhead without compensation.

| Implementation | Profile | Median (ms) | p95 (ms) | σ | Min | Max | Peak RSS (MB) |
|---|---|---:|---:|---:|---:|---:|---:|
| **rust serde_json** | idiomatic | **201** | 211 | 3.6 | 200 | 211 | 12 |
| bun | idiomatic | 260 | 265 | 2.7 | 255 | 265 | 86 |
| rust serde_json (LTO+1cgu) | optimized | 270 | 469 | 82.3 | 196 | 469 | 13 |
| node --max-old=4096 | optimized | 369 | 406 | 12.7 | 356 | 406 | 119 |
| node | idiomatic | 370 | 419 | 16.8 | 358 | 419 | 179 |
| perry (mark-sweep, no lazy) | idiomatic | 384 | 485 | 30.8 | 381 | 485 | 102 |
| kotlin -server -Xmx512m | optimized | 468 | 479 | 7.4 | 457 | 479 | 423 |
| perry (gen-gc + lazy tape) | optimized | 482 | 509 | 10.7 | 468 | 509 | 100 |
| kotlin (kotlinx.serialization) | idiomatic | 588 | 841 | 108.9 | 484 | 841 | 607 |
| c++ -O3 -flto (nlohmann/json) | optimized | 820 | 1249 | 125.4 | 814 | 1249 | 26 |
| go -ldflags="-s -w" -trimpath | optimized | 854 | 1232 | 114.9 | 826 | 1232 | 23 |
| go (encoding/json) | idiomatic | 858 | 930 | 33.0 | 826 | 930 | 23 |
| c++ -O2 (nlohmann/json) | idiomatic | 887 | 901 | 4.7 | 884 | 901 | 25 |
| swift -O (Foundation) | idiomatic | 3735 | 6942 | 1186.6 | 3709 | 6942 | 37 |
| swift -O -wmo (Foundation) | optimized | 3759 | 6279 | 719.6 | 3731 | 6279 | 35 |

**Reading both tables together**: Perry's lazy tape is specifically
optimized for workloads where `parse` is followed by `stringify`
without intermediate iteration. On those workloads it wins decisively
(70 ms, 2.7× over Rust LTO). On workloads that touch every element,
it pays the materialization cost the lazy approach was trying to
avoid — and Perry's `gen-gc + lazy tape` default (482 ms) is actually
SLOWER than its `mark-sweep, no lazy` escape-hatch mode (384 ms).
Rust serde_json with typed structs leads parse-and-iterate at
201 ms because typed deserialization avoids per-value heap
allocations entirely. Bun is the surprise headline: ~260 ms across
both workloads, with single-digit σ — JavaScriptCore's JSON
implementation is genuinely fast and stable.

The honest framing: **Perry leads workloads it's tuned for; loses
gracefully on workloads it isn't.** The `PERRY_JSON_TAPE=0` escape
hatch is right there for the parse-and-iterate case, and on that
flag Perry beats Node and is competitive with Bun.

### Compute microbenches (idiomatic flags)

RUNS=11 per cell. Headline = median ms. Full per-cell stats (median +
p95 + σ + min + max) in
[`polyglot/RESULTS_AUTO.md`](polyglot/RESULTS_AUTO.md) and the
hand-curated [`polyglot/RESULTS.md`](polyglot/RESULTS.md). Lower is
better. **`loop_overhead` and the other flag-aggressiveness probes
have moved to the "Optimization probes" subsection below** — to
avoid presenting them as runtime comparisons when they're really
compiler-flag probes.

| Benchmark           | Perry |  Rust |   C++ |    Go | Swift |  Java |  Node |   Bun |  Python |
|---------------------|------:|------:|------:|------:|------:|------:|------:|------:|--------:|
| fibonacci           |   318 |   330 |   315 |   451 |   406 |   282 |  1022 |   589 |   16054 |
| loop_data_dependent |   231 |   229 |   247 |   132 |   228 |   231 |   233 |   233 |   10750 |
| object_create       |     1 |     0 |     0 |     0 |     0 |     5 |    11 |     6 |     164 |
| nested_loops        |    18 |     8 |     8 |    10 |     8 |    11 |    18 |    21 |     484 |

`fibonacci` (median 318 ms): Perry matches the compiled pack within
3-15 ms; Java's HotSpot JIT is ~11% faster from inlining the
recursive call.

`loop_data_dependent` (median 231 ms for Perry): the genuinely-
non-foldable f64 microbench (multiplicative carry through `sum`
plus array reads, 100M iters; LLVM cannot reorder under reassoc
and cannot vectorize past the sequential dependency — verified at
the asm level, see [`bench.rs`](polyglot/bench.rs#L122)). When the
compiler can't fold, **the playing field FLATTENS**: Perry / Rust /
C++ / Swift / Java / Node / Bun all land within 6 ms of each other
(228-233 ms) because they're all running the same hardware-bound
scalar fmul/fadd loop. **Go is the outlier at 132 ms** — Go's
compiler applies more aggressive scheduling/unrolling on this
specific kernel; we haven't traced exactly why. This bench answers
the legitimate "what does Perry actually do, vs what does its flag
posture do?" question — answer: **competitive with the compiled
pack on genuine compute work, not 7× faster**.

`object_create` (1M iters): median 1 ms — within a tick of native
(Rust/C++/Go/Swift all hit median 0 because their working set fits
in one arena block; Perry hits 1 because gen-GC adds a single
allocation-counter increment per iteration).

`nested_loops` (3000×3000 flat-array sum): cache-bound, not
compute-bound; everyone lands at 8-21 ms.

#### Optimization probes (compiler flag-aggressiveness, not runtime perf)

These four cells are *flag-aggressiveness probes*, not runtime perf
comparisons. They measure whether the compiler applied
**reassoc + IndVarSimplify + autovectorize** to a trivially-foldable
accumulator, NOT how fast the resulting loop actually computes
under load. Perry wins them because TypeScript's `number` semantics
can't observe `reassoc contract` differences (no signalling NaNs,
no fenv, no strict `-0` rules at the operator level), so LLVM's
IndVarSimplify rewrites `sum + 1.0 × N` as an integer induction
variable and the autovectorizer generates `<2 x double>` parallel-
accumulator reductions with interleave count 4. **C++ closes every
one of these gaps with `-O3 -ffast-math`** — same LLVM pipeline,
one flag. See
[`polyglot/RESULTS_OPT.md`](polyglot/RESULTS_OPT.md) for the
per-language flag-tuning sweep that backs out this entire result.

| Benchmark           | Perry |  Rust |   C++ |    Go | Swift |  Java |  Node |   Bun |  Python |
|---------------------|------:|------:|------:|------:|------:|------:|------:|------:|--------:|
| loop_overhead       |    12 |    98 |    98 |    98 |   143 |   100 |    54 |    46 |    3019 |
| math_intensive      |    14 |    48 |    51 |    49 |    50 |    74 |    51 |    51 |    2238 |
| accumulate          |    34 |    98 |    98 |    98 |    98 |   100 |   617 |   100 |    5048 |
| array_read          |     4 |     9 |     9 |    11 |     9 |    12 |    13 |    16 |     342 |
| array_write         |     4 |     7 |     3 |     9 |     2 |     7 |     9 |     6 |     401 |

The companion `loop_data_dependent` (in the headline table above)
shows what Perry looks like on the same kind of kernel WHEN THE
COMPILER CAN'T FOLD: 231 ms, dead-on the 228-233 ms compiled-pack
median. The 12 ms `loop_overhead` and 14 ms `math_intensive`
numbers are real, repeatable, obtained via standard release-mode
builds — but they measure compiler flags, not silicon. A reader
who treats them as "Perry is 7× faster than C++" without reading
this paragraph has been misled by the headline.

**Honest regressions vs the v0.5.164 baseline** (when these benches
were last refreshed, before gen-GC became default):

- `nested_loops` 8 → 18 ms (+10 ms). Caused by the v0.5.237
  generational GC default flip — gen-GC adds per-allocation overhead
  (write-barrier potential, age-bump pass) that's pure cost on
  workloads that don't benefit from it. Set `PERRY_GEN_GC=0` to
  recover the 8 ms baseline.
- `accumulate` 24 → 34 ms (+10 ms). Same root cause; same workaround.
- `object_create` 0 → 1 ms (+1 ms). Same root cause.
- `array_write` / `array_read` 3 → 4 ms each (+1 ms). Within
  measurement noise.
- All other cells (`fibonacci`, `loop_overhead`, `math_intensive`)
  unchanged within ±6 ms of the v0.5.164 baseline.

The trade-off was deliberate: gen-GC's wins on long-running and
allocation-heavy workloads (`test_memory_json_churn` 115 → 91 MB
in v0.5.237) outweigh the small compute-bench regressions, and
the escape hatch is right there. Listed here unapologetically
because the point of this page is to be defensible.

**Tail-latency findings** that median + p95 + σ surfaced (and
best-of-5 had hidden):

- Python `accumulate` median 5052 ms, p95 9388 ms (σ 1454 ms) —
  one run took 9.4 s, ~2× the typical case. Likely GC pressure or
  thermal throttle during a 10 s+ tight loop. The previous best-of-5
  reported "4854 ms" and silently dropped this tail.
- Python `math_intensive`: median 2244, p95 4091 (σ 532). Same
  pattern.
- Swift `-O -wmo` JSON: median 3879 ms, p95 5309 ms (σ 427) —
  Swift's whole-module optimization sometimes spends a long time
  in JSON's reflection pipeline; "optimized" is genuinely noisier
  than `-O` alone (which has σ=73).

These tails are real numbers measured today, not cherry-picked
worst cases. Best-of-N hides them; median + p95 puts them on the
page.

---

## How to read this page

The **compute microbenches** measure compiler choices: loop iteration
throughput, arithmetic latency, sequential array access, recursive
call overhead, object allocation patterns. These are probes into
specific code-generation behavior, not workload simulators. Don't
extrapolate to "language X is N× faster than Y on real applications".

The **JSON benchmarks** are closer to real-world: parse a 1 MB
structured JSON blob (10k records, each with 5 fields including a
nested object and a string array). Two workloads, both reported as
headline tables in TL;DR §A and §B: validate-and-roundtrip
(parse → stringify; no intermediate work) and parse-and-iterate
(parse → sum every record's nested.x → stringify). The two
together catch GC pressure, allocator throughput, encoding/decoding
pipeline cost, AND the cost of touching parsed values vs leaving
them lazy — which separates "Perry's lazy tape avoiding the work"
from "Perry's tape paying overhead it can't amortize".

The **memory benchmarks** are RSS-plateau and GC-aggression regression
tests. They run sustained allocate-and-discard loops for 200k iterations
and assert RSS stays under a per-test ceiling. They catch slow leaks
that microbenchmarks miss.

Every entry below is run twice — **idiomatic** (the language's default
release-mode build, what most projects ship with) and **optimized**
(aggressive flags: LTO, single codegen unit, fast-math where applicable,
etc.). This is intentional. Some readers correctly point out that
"Perry's defaults are themselves aggressive" — so we show every
language's full ceiling, not just its conservative starting point.

---

## 1. JSON polyglot — full data

[`benchmarks/json_polyglot/`](json_polyglot/) — implementation sources +
runner.

### Workload

```typescript
const items = [];
for (let i = 0; i < 10000; i++) {
  items.push({
    id: i,
    name: "item_" + i,
    value: i * 3.14159,
    tags: ["tag_" + (i % 10), "tag_" + (i % 5)],
    nested: { x: i, y: i * 2 }
  });
}
const blob = JSON.stringify(items);  // ~1 MB

// 50 iterations
for (let iter = 0; iter < 50; iter++) {
  const parsed = JSON.parse(blob);
  JSON.stringify(parsed);
}
```

Identical workload in 7 languages: TypeScript (run on Perry / Bun /
Node), Go, Rust, Swift, C++. Each language's implementation lives in
[`bench.<ext>`](json_polyglot/) with the same checksumming logic so
correctness is verifiable.

### Compiler flags used (verbatim)

| Profile | Language | Flags |
|---|---|---|
| optimized | Perry | `cargo build --release -p perry` (LLVM `-O3` equivalent, lazy JSON tape default for ≥1 KB blobs since v0.5.210, gen-GC default ON since v0.5.237) |
| idiomatic | Perry (escape hatch) | `PERRY_GEN_GC=0 PERRY_JSON_TAPE=0` (full mark-sweep, no lazy parse) — included for honesty so a skeptic can see the un-tuned floor |
| idiomatic | Bun | `bun bench.ts` (no flags — Bun is JIT, no compile step) |
| idiomatic | Node | `node --experimental-strip-types bench.ts` |
| optimized | Node | `node --experimental-strip-types --max-old-space-size=4096 bench.ts` |
| idiomatic | Go | `go build` (default) |
| optimized | Go | `go build -ldflags="-s -w" -trimpath` (smaller binary; ~no perf delta — included for completeness, see "honest disclaimers" below) |
| idiomatic | Rust | `cargo build --release` (`opt-level=3`, `lto=false`, `codegen-units=16`) |
| optimized | Rust | `cargo build --profile release-aggressive` (`opt-level=3`, `lto="fat"`, `codegen-units=1`, `panic=abort`, `strip=true`) |
| idiomatic | Swift | `swiftc -O bench.swift` |
| optimized | Swift | `swiftc -O -wmo bench.swift` (whole-module optimization) |
| idiomatic | Kotlin | `java -cp ... BenchKt` (JVM defaults, kotlinx.serialization) |
| optimized | Kotlin | `java -server -Xmx512m -cp ... BenchKt` (server JIT + heap tuning) |
| idiomatic | C++ | `clang++ -std=c++17 -O2` |
| optimized | C++ | `clang++ -std=c++17 -O3 -flto` |

### JSON libraries used

| Language | Library | Why this one |
|---|---|---|
| Perry | built-in `JSON.parse` / `JSON.stringify` (with optional [lazy tape](../docs/json-typed-parse-plan.md)) | Standard JS API, no library to choose |
| Bun / Node | built-in `JSON.parse` / `JSON.stringify` | Standard JS API |
| Go | `encoding/json` | Standard library; what every Go project starts with |
| Rust | `serde_json` (1.0) | The de facto standard; ~ubiquitous in the Rust ecosystem |
| Swift | `Foundation.JSONEncoder` / `JSONDecoder` | Apple's standard |
| Kotlin | `kotlinx.serialization-json` (1.9.0) | The official Kotlin serialization library; uses compile-time-generated (de)serializers, no reflection |
| C++ | nlohmann/json (3.12.0) | The de facto popular C++ JSON library; not the fastest available (RapidJSON / simdjson are faster) but what most projects reach for |

**Faster C++ libraries exist** (RapidJSON, simdjson). We deliberately
benchmark nlohmann/json because that's what real C++ projects use 90%
of the time. If you need to compare against simdjson, it would beat
Perry on time for *parse-only* workloads (it's SIMD-accelerated parse,
no stringify).

### Honest disclaimers on the JSON numbers

- **Perry's `lazy tape` win is workload-specific.** On
  parse-then-iterate-every-element workloads, lazy tape is a net
  loss — it pays the tape build cost without amortizing the
  materialize-on-demand savings. On parse-then-`.length`-or-
  stringify workloads (which this bench is), lazy tape wins
  decisively. See [`audit-lazy-json.md`](../docs/audit-lazy-json.md)
  for the access-pattern matrix.
- **Rust's RSS lead is fundamental.** Rust's serde_json
  deserializes into typed structs (Vec<Item> with stack-laid-out
  fields). Perry, Bun, Node parse into dynamic heap objects (one
  alloc per value). The 8× RSS gap (11 MB Rust vs 85 MB Perry) is
  the cost of dynamic typing — it can't be closed without giving up
  TypeScript's `any` semantics. The fix is to teach Perry's parser
  about typed targets at compile time; tracked as
  [`json-typed-parse-plan.md`](../docs/json-typed-parse-plan.md)
  (Step 2 partially done; more in flight).
- **Go's `optimized` ≈ idiomatic.** `-ldflags="-s -w" -trimpath`
  strips debug info; no measurable perf delta. Included so the
  table doesn't look like Go was unfairly held back. Go has no
  `-ffast-math` flag; `accumulate` and `loop_overhead` deltas in
  the compute table are unrecoverable in stock Go.
- **Swift's slow time is real, not a setup problem.** `-O -wmo`
  is what Swift Package Manager release builds use. The Foundation
  JSON pipeline goes through `Mirror`-based reflection on `Codable`
  types and is genuinely slow on macOS. swift-json is faster; not
  included because this is the standard.
- **Kotlin's RSS is JVM heap reservation, not working-set.** The
  JVM eagerly reserves up to `-Xmx` even when actual heap usage is
  much smaller. `-Xmx512m` gives 423 MB peak RSS; default settings
  reserve more (606 MB observed). The actual JSON working-set in
  Kotlin is comparable to Java/JVM peers. The 423-606 MB RSS
  number is correct for "what the OS sees the process holding"
  but is not a fair comparison of allocator efficiency.
- **Perry's "mark-sweep, no lazy" entry isn't recommended for
  production** — it disables the lazy JSON tape (v0.5.210) and the
  generational GC default (v0.5.237). It exists so you can see the
  untuned floor and compare against it.

---

## 2. Compute microbenches — full data

[`benchmarks/polyglot/`](polyglot/) — 10 implementations across 8
benchmarks. Existing run, last refreshed 2026-04-22 at v0.5.164.

### Idiomatic flags table (current)

See [`RESULTS.md`](polyglot/RESULTS.md) for the full table reproduced
in the TL;DR above. Compiler details:

| Language | Compiler | Idiomatic flag |
|---|---|---|
| Perry | self-hosted Rust, LLVM 22 | `cargo build --release -p perry` |
| Rust | rustc 1.85 stable | `cargo build --release` |
| C++ | clang++ 17 (Apple) | `clang++ -O3 -std=c++17` |
| Go | go 1.21 | `go build` |
| Swift | swiftc 6.0 (Apple) | `swiftc -O` |
| Java | javac 21 + java 21 (HotSpot) | default `java -cp .` |
| Kotlin (JSON only) | kotlinc 2.3.21 | `java -cp ... BenchKt` |
| Node.js | v20 | `node --experimental-strip-types` |
| Bun | 1.3 | `bun` |
| Static Hermes | shermes 0.13 | `shermes -O` (skipped if not installed) |
| Python | 3.12 | `python3` |

Kotlin is JSON-only (not in the compute polyglot table) because the
compute polyglot runner predates Kotlin support; adding it would
require porting the 8-benchmark `bench.kt` to match the existing
`bench.cpp`/`bench.go`/etc. shape. Tracked as a follow-up.

### Optimized flags + delta table

[`RESULTS_OPT.md`](polyglot/RESULTS_OPT.md) holds the full opt-tuning
sweep. Highlights:

- **C++ `-O3 -ffast-math` matches Perry to the millisecond** on
  `loop_overhead` (12 = 12) and `math_intensive` (14 = 14).
- **Rust on stable can't reach Perry on `loop_overhead`** because
  there's no way to expose LLVM's `reassoc` flag on individual
  fadd instructions without nightly's `fadd_fast` intrinsic. With
  manual i64 accumulator + iterator form: 99 → 24 ms (still 2× off).
- **Go cannot close the gap at all**: no `-ffast-math`, no
  `reassoc` flag, the Go compiler doesn't ship that pipeline.
- **Swift `-O -wmo` closes 71-75% of the gap** on
  `loop_overhead` / `math_intensive` / `accumulate`.

### What each microbench actually measures

[`METHODOLOGY.md`](polyglot/METHODOLOGY.md) — full
benchmark-by-benchmark explanation: what's in the inner loop, what
LLVM does with it, what each language's compiler does differently,
why the cell is the number it is. Read this if you suspect any cell
of being unfair.

---

## 3. Memory + GC stability

[`scripts/run_memory_stability_tests.sh`](../scripts/run_memory_stability_tests.sh)
+ [`test-files/test_memory_*.ts`](../test-files/) +
[`test-files/test_gc_*.ts`](../test-files/) — 6 tests × 3 GC mode
combos (default / mark-sweep escape hatch / gen-gc + write
barriers) = 18 runs per CI invocation.

### What each test catches

All numbers from the most recent run on this commit (M1 Max, macOS
26.4). The test asserts RSS stays under the per-test ceiling; the
"Current" column is the actual measured peak.

| Test | What it catches | RSS limit | default | mark-sweep | gen-gc+wb |
|---|---|---:|---:|---:|---:|
| `test_memory_long_lived_loop.ts` | Block-pinning, PARSE_KEY_CACHE leak, tenuring-trap regressions | 100 MB | 54 MB | 54 MB | 54 MB |
| `test_memory_json_churn.ts` | Sparse-cache leak, materialized-tree retention, tape-buffer leak | 200 MB | 91 MB | 91 MB | 91 MB |
| `test_memory_string_churn.ts` | SSO-fast-path-miss alloc, heap-string GC loss | 100 MB | 48 MB | 48 MB | 48 MB |
| `test_memory_closure_churn.ts` | Box leak, closure-env retention, shadow-stack slot leak | 50 MB | 13 MB | 13 MB | 13 MB |
| `test_gc_aggressive_forced.ts` | Conservative-scanner misses, parse-suppressed interleaving, write-barrier mid-mutation | 50 MB | 9 MB | 9 MB | 9 MB |
| `test_gc_deep_recursion.ts` | Stack-scan correctness during deep recursion | 30 MB | 6 MB | 6 MB | 6 MB |

All 18 cells (6 tests × 3 modes) PASS on this commit.

`test_memory_json_churn` dropped from 115 MB → **91 MB** when the
generational-GC default flipped to ON in v0.5.237 (-21%).

### bench_json_roundtrip RSS history

Direct path (`PERRY_JSON_TAPE=0`, 50 iterations of 10k-record parse +
stringify, peak RSS via `/usr/bin/time -l`):

| Version | RSS (MB) | Time (ms) | Change |
|---|---:|---:|---|
| pre-tier-1 (v0.5.193) | ~213 | ~322 | baseline |
| v0.5.198 (threshold 64 MB) | 144 | 364 | tuned initial threshold |
| v0.5.231 (C4b-γ-1, evac no-op) | 109 | ~80 | block-persist + tenuring + arena fixes |
| v0.5.234 (C4b-γ-2, evac live) | 142 | 358 | rebuilt baseline (post-other-changes) |
| v0.5.235 (C4b-δ, dealloc) | 142 | 358 | dealloc fires but peak is pre-first-GC |
| v0.5.236 (C4b-δ-tune, ceiling) | 107 | 358 | trigger ceiling stops step doubling past 64 MB |
| v0.5.237 (gen-gc default ON) | 102 | 372 | minor GC fires by default |
| v0.5.241 (current, this commit) | **102** | **375** | unchanged from v0.5.237; suite re-run for this README |

Default (lazy + gen-gc), the case `bench_json_roundtrip` measures with
no env vars: **66 ms / 85 MB**, currently best in class on time across
every other measured runtime.

### Other Perry benches (best-of-5 minimum across RUNS=5 quick runs, M1 Max, this commit)

| Benchmark | Time (ms) | Peak RSS (MB) |
|---|---:|---:|
| `bench_json_roundtrip` (default, lazy + gen-gc) | 66 | 85 |
| `bench_json_roundtrip` (`PERRY_JSON_TAPE=0`) | 375 | 102 |
| `bench_json_roundtrip` (`PERRY_GEN_GC=0`) | 66 | 85 |
| `bench_json_roundtrip` (both opts off) | 349 | 102 |
| `bench_json_readonly` (default) | 67 | 81 |
| `bench_json_readonly` (`PERRY_JSON_TAPE=0`) | 279 | 103 |
| `07_object_create` | 0 | 6 |
| `12_binary_trees` | 0 | 6 |
| `bench_gc_pressure` | 16 | 25 |
| `04_array_read` | 4 | 211 |
| `05_fibonacci` | 309 | 6 |
| `08_string_concat` | 0 | 6 |

---

## 4. Strengths

Where Perry actually wins, and a one-line "why" per item.

- **JSON validate-and-roundtrip** (parse → stringify, no
  intermediate iteration) — Perry leads on median time at **70 ms**
  (TL;DR §A): 2.7× over Rust serde_json LTO, 3.7× over Bun, 5.7×
  over Node, 6.5× over Kotlin server JIT, 11.3× over C++ -O3 -flto,
  11.9× over Go encoding/json, 53.5× over Swift Foundation. σ=1.7 ms
  — the tightest distribution in the field. The win comes from the
  lazy JSON tape (v0.5.204+): parse builds a 12-byte-per-value tape
  instead of materializing a tree; stringify on an unmutated parse
  memcpy's the original blob. See
  [`json-typed-parse-plan.md`](../docs/json-typed-parse-plan.md).
  Caveat: this is the workload Perry's runtime is specifically
  tuned for. On parse-and-iterate (TL;DR §B), Perry doesn't lead —
  Rust serde_json's typed structs win at 201 ms.
- **f64-arithmetic flag-aggressiveness probes** (`loop_overhead`,
  `math_intensive`, `accumulate`) — Perry 3-8× faster than native on
  these microbenches because TypeScript's `number` semantics let LLVM
  apply `reassoc contract` flags that strict-IEEE languages can't.
  C++ `-O3 -ffast-math` closes this gap; nothing else on the list
  can. **Important framing**: these are *flag-aggressiveness probes*,
  not runtime perf wins on real code. They measure whether the
  compiler folded the loop, not how fast it actually computes. See
  the "Optimization probes" subsection below for the explicit honest
  framing, and the genuinely-non-foldable
  [`loop_data_dependent`](polyglot/bench.rs#L122) for what Perry
  looks like when the compiler *can't* fold.
- **Object allocation in tight loops** (`object_create`, 1M iters) —
  ties native (0 ms). Working set fits in one arena block; GC never
  fires; the inline bump allocator is ~5 instructions per `new`.
- **Generational GC defaults that adapt** (`test_memory_json_churn`
  dropped 115 → 91 MB just from flipping the default) — the
  Bartlett-style mostly-copying generational implementation
  (v0.5.234-237) catches sustained-allocation workloads that pure
  mark-sweep handles poorly.

---

## 5. Weaknesses

The ones we already know about and what's tracked:

- **RSS on dynamic-JSON workloads is high vs typed-struct
  languages.** 85 MB vs Rust's 11 MB on the bench above. Fundamental
  to dynamic typing — every JSON value is a heap NaN-boxed object.
  Mitigation in flight: typed JSON parse (`JSON.parse<T>(blob)`) lets
  the compiler emit packed-keys pre-resolution.
  Step 1 done in v0.5.200.
- **GC pause is stop-the-world.** No concurrent marking. On
  `bench_gc_pressure`, this is 1-2 ms per cycle. On a multi-GB heap
  it would be much more. Tracked as a follow-up in
  [`generational-gc-plan.md`](../docs/generational-gc-plan.md)'s
  "Other parked items" section.
- **No old-generation compaction.** V8, JSC, HotSpot all compact
  old-gen; Perry doesn't. Fragmentation eventually accumulates;
  tracked as a follow-up.
- **Shadow stack is opt-in for the tracer's precision win.** The
  conservative C-stack scan still runs unconditionally because
  shrinking it requires platform-specific FP-chain walking; deferred
  with rationale in
  [`generational-gc-plan.md`](../docs/generational-gc-plan.md)
  §"Deferred follow-ups".
- **TypeScript parity gaps.** 28-test gap-test suite, 18 currently
  passing. Known categorical gaps (lookbehind regex, `console.dir`
  formatting, lone surrogate handling) tracked at
  [`typescript-parity-gaps.md`](../docs/typescript-parity-gaps.md).
- **No JIT.** Compiled code is fixed at build time. JS-engine JIT
  warmup gives V8/JSC a long-tail advantage on iteration-heavy code
  that Perry can't match.
- **Single-threaded by default.** `perry/thread` provides
  parallelMap / spawn but values cross threads via deep-copy
  serialization (no SharedArrayBuffer). Real shared-memory threading
  is not implemented.
- **No incremental / concurrent compilation.** Build time is
  monolithic; incremental rebuilds in v0.5.143's `perry dev` watch
  mode help but full compiles are not yet incremental.

---

## 6. What this page does not measure

- **GC latency / tail latency.** Reported numbers are throughput
  (median wall clock across RUNS=11 invocations). A 99th-percentile
  pause measurement would show Perry's stop-the-world GC at a
  disadvantage vs Go's concurrent collector or HotSpot ZGC.
- **JIT warmup behavior.** JS-family runtimes (Node, Bun) get
  3-iteration warmup before timed iterations to avoid charging them
  for cold-JIT compilation. Real cold-start latency is much worse for
  V8 / JSC than for Perry / Go / Rust binaries.
- **Async / await.** Every benchmark on this page is synchronous.
  Async runtime overhead, event-loop scheduling, microtask draining
  — not measured here.
- **I/O.** No file, network, or DB benchmark. Perry's `perry/thread`
  + tokio integration for HTTP / WebSocket / DB is benchmarked
  separately (see [`docs/`](../docs/) — partial).
- **Realistic application workloads.** Microbenches are probes,
  not workload simulators. The "Perry is X× faster than Y" claim
  is only true on the specific workload shape measured.
- **Memory pressure under contention.** All benches run on an
  otherwise-idle machine. Behavior under co-located-tenant pressure
  is not measured.
- **Compile time / binary size.** Perry compiles slower than Go (Go
  is famously fast at compile-time). Binary size is ~1 MB for hello
  world; comparable to Go but bigger than Rust release binaries with
  panic=abort + strip.

---

## 7. Reproducing

### JSON polyglot

```bash
# In repo root, build Perry:
cargo build --release -p perry-runtime -p perry-stdlib -p perry

# Install the C++ JSON dependency (macOS):
brew install nlohmann-json

# Run the polyglot suite:
cd benchmarks/json_polyglot
./run.sh             # RUNS=11 default (median + p95 + σ + min + max)
RUNS=21 ./run.sh     # 21 runs for tighter intervals
```

Outputs `benchmarks/json_polyglot/RESULTS.md` with the full table.

### Compute microbenches

```bash
cd benchmarks/polyglot
./run_all.sh         # RUNS=11 default (median + p95 + σ + min + max)
./run_all.sh 21      # 21 runs for tighter intervals
```

Missing language toolchains show as `-` in the table; the script
degrades gracefully.

### Memory stability tests

```bash
bash scripts/run_memory_stability_tests.sh
```

Runs 18 test combinations (6 tests × 3 GC modes), prints PASS/FAIL +
RSS per cell. Wired into CI via `.github/workflows/test.yml`.

---

## 8. Design / implementation references

- [`docs/generational-gc-plan.md`](../docs/generational-gc-plan.md) —
  the GC architecture: phases A-D, write barriers, evacuation,
  conservative pinning, plus the academic + industry lineage
  appendix (Bartlett 1988, Ungar 1984, Cheney 1970, etc.).
- [`docs/json-typed-parse-plan.md`](../docs/json-typed-parse-plan.md) —
  the JSON pipeline design: tape format, lazy materialization,
  typed-parse plan.
- [`docs/audit-lazy-json.md`](../docs/audit-lazy-json.md) — external
  reviewer reference for the lazy-parse correctness guarantees +
  access-pattern matrix.
- [`docs/memory-perf-roadmap.md`](../docs/memory-perf-roadmap.md) —
  RSS optimization roadmap (tier 1: NaN-boxing, tier 2: SSO, tier 3:
  generational GC).
- [`docs/sso-migration-plan.md`](../docs/sso-migration-plan.md) —
  Small String Optimization rollout sequencing.
- [`benchmarks/polyglot/METHODOLOGY.md`](polyglot/METHODOLOGY.md) —
  per-microbenchmark explanation, compiler versions, why each cell
  is the number it is.
- [`CHANGELOG.md`](../CHANGELOG.md) — every version, every change,
  with measured impact where applicable.

If you spot something that looks unfair, biased, or wrong: open an
issue at https://github.com/PerryTS/perry/issues with the
benchmark name, your alternative implementation, and the toolchain
versions you ran with. The point of this page is to be defensible,
not to win. Numbers that don't survive scrutiny don't belong here.
