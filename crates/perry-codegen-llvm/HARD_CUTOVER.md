# Phase K — Hard Cutover Dry-Run

When all four parity gates close, this is the **complete change list**
for deleting the Cranelift backend and renaming `perry-codegen-llvm`
to `perry-codegen`. The work here is mostly mechanical because the
soft cutover (commit `5b104fd`) already moved all driver dispatch
through the LLVM path.

## Source-tree dependencies on `perry_codegen` (the Cranelift crate)

`grep -rn "perry_codegen::" crates/`:

| Site | Purpose | Replacement |
|---|---|---|
| `crates/perry/src/commands/compile.rs:3454` | `perry_codegen::set_i18n_table(...)` (main thread setup) | DELETE — LLVM threads i18n through `CompileOptions::i18n_table` (`codegen.rs:154`) |
| `crates/perry/src/commands/compile.rs:4477` | `perry_codegen::set_i18n_table(...)` (rayon worker setup) | DELETE — LLVM does not need a thread-local table; the per-job `CompileOptions` clones the data |
| `crates/perry/src/commands/compile.rs:4480` | `perry_codegen::Compiler::new(target)` (Cranelift compile entry) | DELETE — entire `else` arm of the `if use_llvm_backend` block (lines 4474–4729 approx) |
| `crates/perry/src/commands/compile.rs:4826` | `perry_codegen::generate_stub_object(...)` (missing-symbol stubs) | REPLACE → `perry_codegen_llvm::stubs::generate_stub_object(...)` ✓ (this commit) |
| `crates/perry/src/commands/compile.rs:4965` | `perry_codegen::generate_stub_object(...)` (failed-module init stubs) | REPLACE → `perry_codegen_llvm::stubs::generate_stub_object(...)` ✓ (this commit) |

After the renames, **zero** `perry_codegen::` references remain outside
of `crates/perry-codegen/` itself, which gets deleted entirely.

## Workspace surface

`Cargo.toml` deletions:

- `members` line: `"crates/perry-codegen"` (line 9)
- `default-members` line: `"crates/perry-codegen"` (line 40)
- `[workspace.dependencies]`: `perry-codegen = { path = "crates/perry-codegen" }` (line 162)
- `[workspace.dependencies]`: the entire `cranelift*` block (lines 102–107) — only Cranelift uses these

## CLI surface

`crates/perry/src/commands/compile.rs`:

- Lines 60–67: `pub backend: String` and the `#[arg(long, default_value = "llvm")]` attribute → DELETE the field entirely
- Lines 4205–4218: `let use_llvm_backend = ...;` and the deprecation warning → DELETE
- Lines 4232 / 4473: the `if use_llvm_backend { ... return Ok(...) }` wrapper → unwrap the body so the LLVM path becomes unconditional
- Lines 4474–~4729: the entire Cranelift fallback closure body → DELETE

## Blockers (must be resolved before cutting)

| # | Blocker | Status |
|---|---------|--------|
| 1 | All 22 `test_gap_*.ts` MATCH on `--backend llvm` | 🚧 4 parallel agents closing typed arrays / symbols / async generators / crypto buffers |
| 2 | `run_parity_tests.sh` fully green (excluding determinism trio) | 🚧 currently 108 MATCH / 10 DIFF / 1 CRASH / 1 COMPILE_FAIL |
| 3 | LLVM ≤ 105% Cranelift on every benchmark | ✅ already cleared (−44% Phase 2.1 + −31% Phase J bitcode) |
| 4 | Bitcode-link mode passes both gates above | ✅ landed v0.4.90, no regressions reported |
| 5 | LLVM-side `generate_stub_object` exists | ✅ this commit (`crates/perry-codegen-llvm/src/stubs.rs`) |

Only #1 and #2 still block. Both are agent work in progress.

## Rename — `perry-codegen-llvm` → `perry-codegen`

After deleting the old crate, the new crate's name should match the
plan (the LLVM backend IS the codegen backend going forward). Steps:

1. `git mv crates/perry-codegen-llvm crates/perry-codegen`
2. In `crates/perry-codegen/Cargo.toml`: `name = "perry-codegen"`
3. In root `Cargo.toml`:
   - `members`: `"crates/perry-codegen-llvm"` → `"crates/perry-codegen"`
   - `default-members`: same rename
   - `[workspace.dependencies]`: `perry-codegen-llvm = { path = "crates/perry-codegen-llvm" }` → `perry-codegen = { path = "crates/perry-codegen" }`
4. `grep -rln "perry_codegen_llvm" crates/ | xargs sed -i '' 's/perry_codegen_llvm/perry_codegen/g'`
5. `grep -rln "perry-codegen-llvm" crates/ docs/ benchmarks/ | xargs sed -i '' 's/perry-codegen-llvm/perry-codegen/g'`

The grep/sed sweeps are safe because the **old** `perry-codegen` is
gone in step 1, so no name collision is possible.

## Files renamed in docs / scripts

A non-exhaustive grep finds these references that should be updated
together with the rename:

- `CLAUDE.md` — architecture table (`perry-codegen-llvm` → `perry-codegen`)
- `README.md:629` — directory tree
- `benchmarks/compare_backends.sh` — header text only (functional logic
  goes away with the Cranelift backend)
- `benchmarks/llvm_vs_cranelift.md` — keep as a historical record but
  add a note that the `Cranelift` column reflects pre-cutover state

## After-cutover validation

```bash
# 1. Workspace builds
cargo build --release -p perry -p perry-runtime -p perry-stdlib

# 2. No dangling references
grep -rn "perry_codegen_llvm\|perry-codegen-llvm" crates/  # → empty
grep -rn "perry_codegen::" crates/                          # → empty
grep -rn "Cranelift\|cranelift" crates/                     # → empty (apart from benchmarks/llvm_vs_cranelift.md)

# 3. Sweep is still green
./run_parity_tests.sh

# 4. Sample real-world programs
./target/release/perry compile example-code/http-server/main.ts -o /tmp/http-server
./target/release/perry compile example-code/blockchain-demo/main.ts -o /tmp/bc

# 5. cargo test still works for the renamed crate
cargo test -p perry-codegen
```

## Rollback strategy

If the parity sweep regresses inside this single commit, the rollback
is `git revert` plus restoring the workspace member entries. The
Cranelift backend is preserved on `main` until the hard cutover lands,
so the worst case is one revert.
