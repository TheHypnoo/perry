# JSON Polyglot Benchmark Results

**Runs per cell:** 11 · **Pinning:** macOS scheduler hint (taskpolicy -t 0 -l 0 — P-core preferred via throughput/latency tiers, NOT strict affinity)
**Hardware:** Darwin 25.4.0 arm64 on MacBookPro.
**Date:** 2026-04-25.

Two workloads, each language listed twice (idiomatic / optimized flag profile).
Median wall-clock time is the headline number; p95, σ (population stddev),
min, and max are reported per cell so noise is visible. Lower is better.

## JSON validate-and-roundtrip

Per iteration: parse → stringify → discard. The unmutated parse lets
Perry's lazy tape (v0.5.204+) memcpy the original blob bytes for
stringify, which is why Perry's headline number on this workload is so
low — the lazy path can avoid materializing the parse tree entirely.
10k records, ~1 MB blob, 50 iterations per run.

| Implementation | Profile | Median (ms) | p95 (ms) | σ | Min | Max | Peak RSS (MB) |
|---|---|---:|---:|---:|---:|---:|---:|
| perry (gen-gc + lazy tape) | optimized | 70 | 74 | 1.7 | 68 | 74 | 85 |
| rust serde_json (LTO+1cgu) | optimized | 187 | 318 | 40.6 | 184 | 318 | 13 |
| rust serde_json | idiomatic | 202 | 208 | 2.2 | 198 | 208 | 12 |
| bun (default) | idiomatic | 260 | 270 | 5.0 | 255 | 270 | 84 |
| perry (mark-sweep, no lazy) | idiomatic | 366 | 393 | 11.8 | 359 | 393 | 102 |
| node --max-old=4096 | optimized | 395 | 761 | 123.4 | 381 | 761 | 182 |
| node (default) | idiomatic | 396 | 486 | 28.8 | 379 | 486 | 182 |
| kotlin -server -Xmx512m | optimized | 457 | 490 | 12.8 | 451 | 490 | 426 |
| kotlin (kotlinx.serialization) | idiomatic | 484 | 495 | 7.1 | 469 | 495 | 608 |
| c++ -O3 -flto (nlohmann/json) | optimized | 788 | 807 | 6.5 | 780 | 807 | 25 |
| go -ldflags="-s -w" -trimpath | optimized | 823 | 885 | 18.7 | 812 | 885 | 22 |
| go (encoding/json) | idiomatic | 831 | 1123 | 92.9 | 811 | 1123 | 23 |
| c++ -O2 (nlohmann/json) | idiomatic | 872 | 1550 | 304.2 | 858 | 1550 | 28 |
| swift -O (Foundation) | idiomatic | 3747 | 5108 | 391.2 | 3713 | 5108 | 34 |
| swift -O -wmo (Foundation) | optimized | 3778 | 4395 | 178.1 | 3763 | 4395 | 35 |

## JSON parse-and-iterate

Per iteration: parse → sum every record's nested.x (touches every element)
→ stringify. The full-tree iteration FORCES Perry's lazy tape to
materialize, so this is the honest comparison for workloads that touch
JSON content. 10k records, ~1 MB blob, 50 iterations per run.

| Implementation | Profile | Median (ms) | p95 (ms) | σ | Min | Max | Peak RSS (MB) |
|---|---|---:|---:|---:|---:|---:|---:|
| rust serde_json | idiomatic | 201 | 211 | 3.6 | 200 | 211 | 12 |
| bun (default) | idiomatic | 260 | 265 | 2.7 | 255 | 265 | 86 |
| rust serde_json (LTO+1cgu) | optimized | 270 | 469 | 82.3 | 196 | 469 | 13 |
| node --max-old=4096 | optimized | 369 | 406 | 12.7 | 356 | 406 | 119 |
| node (default) | idiomatic | 370 | 419 | 16.8 | 358 | 419 | 179 |
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
