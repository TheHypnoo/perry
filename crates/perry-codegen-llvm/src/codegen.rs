//! HIR → LLVM IR compilation entry point.
//!
//! Public contract:
//!
//! ```ignore
//! let opts = CompileOptions { target: None, is_entry_module: true };
//! let object_bytes: Vec<u8> = perry_codegen_llvm::compile_module(&hir, opts)?;
//! ```
//!
//! The returned bytes are a regular object file produced by `clang -c`.
//! Perry's existing linking stage in `crates/perry/src/commands/compile.rs`
//! picks them up identically to the Cranelift output.
//!
//! ## Phase 2 scope
//!
//! - User functions with typed `double` ABI (all params and returns are
//!   `double`; no NaN-boxing yet)
//! - Recursive and forward calls via `FuncRef`
//! - If/else with straight-line or terminating branches
//! - `let`/`const` numeric locals (alloca + mem2reg pattern)
//! - Binary arithmetic (add/sub/mul/div/mod)
//! - Comparisons (lifted to 0.0/1.0 doubles)
//! - `console.log(<numeric expr>)` sink at statement level
//!
//! Anything richer (strings, objects, closures, loops, Date.now, classes,
//! imports) errors with an actionable message from `expr::lower_expr` or
//! `stmt::lower_stmt`.

use std::collections::HashMap;

use anyhow::{anyhow, Context, Result};
use perry_hir::{Function, Module as HirModule};

use crate::expr::FnCtx;
use crate::module::LlModule;
use crate::runtime_decls;
use crate::stmt;
use crate::types::{DOUBLE, I32, LlvmType};

/// Options mirrored from the Cranelift backend's setter API.
#[derive(Debug, Clone, Default)]
pub struct CompileOptions {
    /// Target triple override. `None` uses the host default.
    pub target: Option<String>,
    /// Whether this module is the program entry point. When true, codegen
    /// emits a `main` function that calls `js_gc_init` and then the module's
    /// top-level statements.
    pub is_entry_module: bool,
}

/// Compile a Perry HIR module to an object file via LLVM IR.
pub fn compile_module(hir: &HirModule, opts: CompileOptions) -> Result<Vec<u8>> {
    let triple = opts.target.clone().unwrap_or_else(default_target_triple);

    let mut llmod = LlModule::new(&triple);
    runtime_decls::declare_phase1(&mut llmod);

    // Phase 2 still only supports single-file entry modules.
    if !opts.is_entry_module {
        return Err(anyhow!(
            "perry-codegen-llvm Phase 2 only supports the entry module; \
             non-entry module '{}' is not yet supported",
            hir.name
        ));
    }
    if !hir.imports.is_empty() {
        return Err(anyhow!(
            "perry-codegen-llvm Phase 2 does not support imports; module '{}' has {} imports",
            hir.name,
            hir.imports.len()
        ));
    }
    if !hir.classes.is_empty() {
        return Err(anyhow!(
            "perry-codegen-llvm Phase 2 does not support classes; module '{}' has {} classes",
            hir.name,
            hir.classes.len()
        ));
    }

    // Resolve user function names up-front so body lowering can emit
    // forward/recursive calls without worrying about emission order.
    let mut func_names: HashMap<u32, String> = HashMap::new();
    for f in &hir.functions {
        func_names.insert(f.id, llvm_fn_name(&f.name));
    }

    // Lower each user function into the module.
    for f in &hir.functions {
        compile_function(&mut llmod, f, &func_names)
            .with_context(|| format!("lowering function '{}'", f.name))?;
    }

    // Emit `int main()` that bootstraps GC and runs init statements.
    compile_main(&mut llmod, hir, &func_names)
        .with_context(|| format!("lowering main of module '{}'", hir.name))?;

    let ll_text = llmod.to_ir();
    log::debug!(
        "perry-codegen-llvm: emitted {} bytes of LLVM IR for '{}'",
        ll_text.len(),
        hir.name
    );
    crate::linker::compile_ll_to_object(&ll_text, opts.target.as_deref())
}

/// Compile a single user function into the module.
fn compile_function(
    llmod: &mut LlModule,
    f: &Function,
    func_names: &HashMap<u32, String>,
) -> Result<()> {
    let llvm_name = func_names
        .get(&f.id)
        .cloned()
        .ok_or_else(|| anyhow!("function name not resolved for {}", f.name))?;

    // Phase 2 assumes all user-function params are `double`. Parameter
    // registers are named `%arg{LocalId}` so the body can store them into
    // alloca slots keyed by the same HIR LocalId.
    let params: Vec<(LlvmType, String)> = f
        .params
        .iter()
        .map(|p| (DOUBLE, format!("%arg{}", p.id)))
        .collect();

    let lf = llmod.define_function(&llvm_name, DOUBLE, params);
    let _ = lf.create_block("entry");

    // Store each param into an alloca slot, collecting LocalId → slot
    // mappings. We release the &mut LlBlock at scope end before handing
    // the function over to the FnCtx lowering pass.
    let locals: HashMap<u32, String> = {
        let blk = lf.block_mut(0).unwrap();
        let mut map = HashMap::new();
        for p in &f.params {
            let slot = blk.alloca(DOUBLE);
            blk.store(DOUBLE, &format!("%arg{}", p.id), &slot);
            map.insert(p.id, slot);
        }
        map
    };

    let mut ctx = FnCtx {
        func: lf,
        locals,
        current_block: 0,
        func_names,
    };
    stmt::lower_stmts(&mut ctx, &f.body)
        .with_context(|| format!("lowering body of '{}'", f.name))?;

    // Defensive: a well-typed numeric function always returns via an
    // explicit `return`, but we emit `ret double 0.0` as a fallback so
    // the LLVM verifier doesn't reject a missing terminator.
    if !ctx.block().is_terminated() {
        ctx.block().ret(DOUBLE, "0.0");
    }
    Ok(())
}

/// Emit `int main() { js_gc_init(); <init stmts>; return 0; }`.
fn compile_main(
    llmod: &mut LlModule,
    hir: &HirModule,
    func_names: &HashMap<u32, String>,
) -> Result<()> {
    let main = llmod.define_function("main", I32, vec![]);
    let _ = main.create_block("entry");
    {
        let blk = main.block_mut(0).unwrap();
        blk.call_void("js_gc_init", &[]);
    }

    let mut ctx = FnCtx {
        func: main,
        locals: HashMap::new(),
        current_block: 0,
        func_names,
    };
    stmt::lower_stmts(&mut ctx, &hir.init)
        .with_context(|| format!("lowering init statements of module '{}'", hir.name))?;

    // `main` returns i32, but stmt lowering emits `ret double` for explicit
    // returns. Phase 2 doesn't allow explicit returns at top level, so we
    // just append `ret i32 0` if the block didn't terminate.
    if !ctx.block().is_terminated() {
        ctx.block().ret(I32, "0");
    }
    Ok(())
}

/// Mangle a HIR function name into an LLVM symbol.
///
/// We prefix with `perry_fn_` to avoid colliding with runtime symbols like
/// `main`, `js_console_log_*`, or the C stdlib. Non-alphanumeric characters
/// are replaced with underscores because LLVM symbol names are restrictive.
fn llvm_fn_name(hir_name: &str) -> String {
    let sanitized: String = hir_name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' })
        .collect();
    format!("perry_fn_{}", sanitized)
}

/// Host default triple.
fn default_target_triple() -> String {
    if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        "arm64-apple-macosx15.0.0".to_string()
    } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        "x86_64-apple-macosx15.0.0".to_string()
    } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        "x86_64-unknown-linux-gnu".to_string()
    } else if cfg!(all(target_os = "linux", target_arch = "aarch64")) {
        "aarch64-unknown-linux-gnu".to_string()
    } else {
        "arm64-apple-macosx15.0.0".to_string()
    }
}
