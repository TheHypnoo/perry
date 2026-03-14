//! HIR → WebAssembly bytecode emitter
//!
//! Translates HIR modules to WebAssembly binary format using wasm-encoder.
//! All JSValues are represented as f64 using NaN-boxing (matching perry-runtime).
//! Runtime operations (strings, console, objects) are imported from a JS bridge.

use perry_hir::ir::*;
use perry_types::{FuncId, LocalId, GlobalId};
use std::collections::BTreeMap;
use wasm_encoder::{
    CodeSection, DataSection, EntityType, ExportKind, ExportSection, Function,
    FunctionSection, Ieee64, ImportSection, Instruction, MemorySection, MemoryType,
    Module, TypeSection, ValType, GlobalSection, GlobalType,
};

/// Helper: create an F64Const instruction from raw f64 bits
fn f64_const(val: f64) -> Instruction<'static> {
    Instruction::F64Const(Ieee64::from(val))
}

/// Helper: create an F64Const instruction from NaN-boxed tag bits
fn f64_const_bits(bits: u64) -> Instruction<'static> {
    Instruction::F64Const(Ieee64::from(f64::from_bits(bits)))
}

// NaN-boxing constants (must match perry-runtime and wasm_runtime.js)
const STRING_TAG: u64 = 0x7FFF;
const TAG_UNDEFINED: u64 = 0x7FFC_0000_0000_0001;
const TAG_NULL: u64 = 0x7FFC_0000_0000_0002;
const TAG_FALSE: u64 = 0x7FFC_0000_0000_0003;
const TAG_TRUE: u64 = 0x7FFC_0000_0000_0004;

/// Import function indices (must match the order imports are added)
#[derive(Clone, Copy)]
struct RuntimeImports {
    string_new: u32,
    console_log: u32,
    console_warn: u32,
    console_error: u32,
    string_concat: u32,
    js_add: u32,
    string_eq: u32,
    string_len: u32,
    jsvalue_to_string: u32,
    is_truthy: u32,
    js_strict_eq: u32,
    math_floor: u32,
    math_ceil: u32,
    math_round: u32,
    math_abs: u32,
    math_sqrt: u32,
    math_pow: u32,
    math_random: u32,
    math_log: u32,
    date_now: u32,
    js_typeof: u32,
    math_min: u32,
    math_max: u32,
    parse_int: u32,
    parse_float: u32,
}

/// Compile HIR modules to a WebAssembly binary.
pub fn compile_to_wasm(modules: &[(String, perry_hir::ir::Module)]) -> Vec<u8> {
    let mut emitter = WasmModuleEmitter::new();
    emitter.compile(modules)
}

struct WasmModuleEmitter {
    /// String literal table: content → (string_id, offset, length)
    string_table: Vec<(String, u32, u32)>, // (content, offset, len)
    string_map: BTreeMap<String, u32>,      // content → string_id
    string_data: Vec<u8>,                   // packed string bytes
    /// Type section entries: (params, results)
    types: Vec<(Vec<ValType>, Vec<ValType>)>,
    type_map: BTreeMap<(Vec<ValType>, Vec<ValType>), u32>,
    /// Function index mapping
    func_map: BTreeMap<FuncId, u32>,
    /// Import count (import functions come first in the index space)
    num_imports: u32,
    /// Runtime import indices
    rt: Option<RuntimeImports>,
    /// Global variable mapping: GlobalId → wasm global index
    global_map: BTreeMap<GlobalId, u32>,
    num_globals: u32,
}

impl WasmModuleEmitter {
    fn new() -> Self {
        Self {
            string_table: Vec::new(),
            string_map: BTreeMap::new(),
            string_data: Vec::new(),
            types: Vec::new(),
            type_map: BTreeMap::new(),
            func_map: BTreeMap::new(),
            num_imports: 0,
            rt: None,
            global_map: BTreeMap::new(),
            num_globals: 0,
        }
    }

    /// Intern a string literal, returning its string_id.
    fn intern_string(&mut self, s: &str) -> u32 {
        if let Some(&id) = self.string_map.get(s) {
            return id;
        }
        let id = self.string_table.len() as u32;
        let offset = self.string_data.len() as u32;
        let bytes = s.as_bytes();
        let len = bytes.len() as u32;
        self.string_data.extend_from_slice(bytes);
        self.string_table.push((s.to_string(), offset, len));
        self.string_map.insert(s.to_string(), id);
        id
    }

    /// Get or create a function type index for the given signature.
    fn get_type_idx(&mut self, params: Vec<ValType>, results: Vec<ValType>) -> u32 {
        let key = (params.clone(), results.clone());
        if let Some(&idx) = self.type_map.get(&key) {
            return idx;
        }
        let idx = self.types.len() as u32;
        self.types.push((params, results));
        self.type_map.insert(key, idx);
        idx
    }

    fn compile(&mut self, modules: &[(String, perry_hir::ir::Module)]) -> Vec<u8> {
        // First pass: collect all string literals
        for (_, module) in modules {
            self.collect_strings(module);
        }

        // Register runtime import types and get type indices
        // All imports use f64 for JSValues
        let t_void = self.get_type_idx(vec![], vec![]);
        let t_i32_i32_void = self.get_type_idx(vec![ValType::I32, ValType::I32], vec![]);
        let t_f64_void = self.get_type_idx(vec![ValType::F64], vec![]);
        let t_f64_f64_f64 = self.get_type_idx(vec![ValType::F64, ValType::F64], vec![ValType::F64]);
        let t_f64_f64_i32 = self.get_type_idx(vec![ValType::F64, ValType::F64], vec![ValType::I32]);
        let t_f64_f64 = self.get_type_idx(vec![ValType::F64], vec![ValType::F64]);
        let t_f64_i32 = self.get_type_idx(vec![ValType::F64], vec![ValType::I32]);
        let t_void_f64 = self.get_type_idx(vec![], vec![ValType::F64]);

        // Add runtime imports (order matters — defines function indices)
        let mut import_idx: u32 = 0;
        let mut next_import = || { let i = import_idx; import_idx += 1; i };

        let rt = RuntimeImports {
            string_new: next_import(),       // (i32, i32) -> void
            console_log: next_import(),      // (f64) -> void
            console_warn: next_import(),     // (f64) -> void
            console_error: next_import(),    // (f64) -> void
            string_concat: next_import(),    // (f64, f64) -> f64
            js_add: next_import(),           // (f64, f64) -> f64
            string_eq: next_import(),        // (f64, f64) -> i32
            string_len: next_import(),       // (f64) -> f64
            jsvalue_to_string: next_import(),// (f64) -> f64
            is_truthy: next_import(),        // (f64) -> i32
            js_strict_eq: next_import(),     // (f64, f64) -> i32
            math_floor: next_import(),       // (f64) -> f64
            math_ceil: next_import(),        // (f64) -> f64
            math_round: next_import(),       // (f64) -> f64
            math_abs: next_import(),         // (f64) -> f64
            math_sqrt: next_import(),        // (f64) -> f64
            math_pow: next_import(),         // (f64, f64) -> f64
            math_random: next_import(),      // () -> f64
            math_log: next_import(),         // (f64) -> f64
            date_now: next_import(),         // () -> f64
            js_typeof: next_import(),        // (f64) -> f64
            math_min: next_import(),         // (f64, f64) -> f64
            math_max: next_import(),         // (f64, f64) -> f64
            parse_int: next_import(),        // (f64) -> f64
            parse_float: next_import(),      // (f64) -> f64
        };
        self.num_imports = import_idx;
        self.rt = Some(rt);

        // Map the import types
        let import_types: Vec<u32> = vec![
            t_i32_i32_void,  // string_new
            t_f64_void,      // console_log
            t_f64_void,      // console_warn
            t_f64_void,      // console_error
            t_f64_f64_f64,   // string_concat
            t_f64_f64_f64,   // js_add
            t_f64_f64_i32,   // string_eq
            t_f64_f64,       // string_len  (f64 -> f64)
            t_f64_f64,       // jsvalue_to_string
            t_f64_i32,       // is_truthy
            t_f64_f64_i32,   // js_strict_eq
            t_f64_f64,       // math_floor
            t_f64_f64,       // math_ceil
            t_f64_f64,       // math_round
            t_f64_f64,       // math_abs
            t_f64_f64,       // math_sqrt
            t_f64_f64_f64,   // math_pow
            t_void_f64,      // math_random
            t_f64_f64,       // math_log
            t_void_f64,      // date_now
            t_f64_f64,       // js_typeof
            t_f64_f64_f64,   // math_min
            t_f64_f64_f64,   // math_max
            t_f64_f64,       // parse_int  (f64 -> f64, expects string)
            t_f64_f64,       // parse_float
        ];

        let import_names = [
            "string_new", "console_log", "console_warn", "console_error",
            "string_concat", "js_add", "string_eq", "string_len",
            "jsvalue_to_string", "is_truthy", "js_strict_eq",
            "math_floor", "math_ceil", "math_round", "math_abs", "math_sqrt",
            "math_pow", "math_random", "math_log", "date_now", "js_typeof",
            "math_min", "math_max", "parse_int", "parse_float",
        ];

        // Second pass: register all user function types and assign indices
        let mut user_func_idx = self.num_imports;

        // __init_strings function
        let init_strings_idx = user_func_idx;
        let init_strings_type = t_void;
        user_func_idx += 1;

        // Register user functions from all modules
        for (_, module) in modules {
            for func in &module.functions {
                let param_count = func.params.len();
                let params = vec![ValType::F64; param_count];
                let results = if func.body.iter().any(|s| has_return(s)) || func.name == "main" {
                    // Functions that return values
                    vec![ValType::F64]
                } else {
                    vec![]
                };
                let type_idx = self.get_type_idx(params, results);
                let _ = type_idx;
                self.func_map.insert(func.id, user_func_idx);
                user_func_idx += 1;
            }
        }

        // _start function (entry point)
        let start_idx = user_func_idx;
        let start_type = t_void;
        user_func_idx += 1;

        // Register globals from all modules
        for (_, module) in modules {
            for global in &module.globals {
                self.global_map.insert(global.id, self.num_globals);
                self.num_globals += 1;
            }
        }

        // Build the WASM module
        let mut wasm_module = Module::new();

        // --- Type section ---
        let mut type_section = TypeSection::new();
        for (params, results) in &self.types {
            type_section.ty().function(
                params.iter().copied(),
                results.iter().copied(),
            );
        }
        wasm_module.section(&type_section);

        // --- Import section ---
        let mut import_section = ImportSection::new();
        for (i, name) in import_names.iter().enumerate() {
            import_section.import("rt", name, EntityType::Function(import_types[i]));
        }
        wasm_module.section(&import_section);

        // --- Function section (declares type indices for each defined function) ---
        let mut func_section = FunctionSection::new();
        // __init_strings
        func_section.function(init_strings_type);
        // User functions
        for (_, module) in modules {
            for func in &module.functions {
                let param_count = func.params.len();
                let params = vec![ValType::F64; param_count];
                let results = if func.body.iter().any(|s| has_return(s)) || func.name == "main" {
                    vec![ValType::F64]
                } else {
                    vec![]
                };
                let type_idx = self.get_type_idx(params, results);
                func_section.function(type_idx);
            }
        }
        // _start
        func_section.function(start_type);
        wasm_module.section(&func_section);

        // --- Memory section ---
        let mut mem_section = MemorySection::new();
        let pages = ((self.string_data.len() + 65535) / 65536).max(1) as u64;
        mem_section.memory(MemoryType {
            minimum: pages,
            maximum: None,
            memory64: false,
            shared: false,
            page_size_log2: None,
        });
        wasm_module.section(&mem_section);

        // --- Global section (mutable f64 globals for module-level variables) ---
        if self.num_globals > 0 {
            let mut global_section = GlobalSection::new();
            for _ in 0..self.num_globals {
                global_section.global(
                    GlobalType {
                        val_type: ValType::F64,
                        mutable: true,
                        shared: false,
                    },
                    &wasm_encoder::ConstExpr::f64_const(Ieee64::from(f64::from_bits(TAG_UNDEFINED))),
                );
            }
            wasm_module.section(&global_section);
        }

        // --- Export section ---
        let mut export_section = ExportSection::new();
        export_section.export("_start", ExportKind::Func, start_idx);
        export_section.export("memory", ExportKind::Memory, 0);
        wasm_module.section(&export_section);

        // --- DataCount section (required before Code when Data section exists) ---
        if !self.string_data.is_empty() {
            wasm_module.section(&wasm_encoder::DataCountSection { count: 1 });
        }

        // --- Code section ---
        let mut code_section = CodeSection::new();

        // __init_strings: register all string literals with the JS runtime
        {
            let mut func = Function::new(vec![]);
            for (_content, offset, len) in &self.string_table {
                func.instruction(&Instruction::I32Const(*offset as i32));
                func.instruction(&Instruction::I32Const(*len as i32));
                func.instruction(&Instruction::Call(rt.string_new));
            }
            func.instruction(&Instruction::End);
            code_section.function(&func);
        }

        // User functions
        for (_, module) in modules {
            for hir_func in &module.functions {
                let func = self.compile_function(hir_func);
                code_section.function(&func);
            }
        }

        // _start: call __init_strings, then execute module init code
        {
            // Collect all init statements to determine locals needed
            let mut init_locals = BTreeMap::new();
            for (_, module) in modules {
                for stmt in &module.init {
                    if let Stmt::Let { id, .. } = stmt {
                        init_locals.insert(*id, init_locals.len() as u32);
                    }
                }
            }

            let num_locals = init_locals.len();
            let locals = if num_locals > 0 {
                vec![(num_locals as u32, ValType::F64)]
            } else {
                vec![]
            };
            let mut func = Function::new(locals);

            // Call __init_strings first
            func.instruction(&Instruction::Call(init_strings_idx));

            // Initialize globals
            for (_, module) in modules {
                for global in &module.globals {
                    if let Some(init) = &global.init {
                        let mut ctx = FuncEmitCtx::new(self, &init_locals);
                        ctx.emit_expr(&mut func, init);
                        let gidx = self.global_map[&global.id];
                        func.instruction(&Instruction::GlobalSet(gidx));
                    } else if global.name == "__platform__" {
                        // Web platform ID = 5
                        func.instruction(&f64_const(5.0));
                        let gidx = self.global_map[&global.id];
                        func.instruction(&Instruction::GlobalSet(gidx));
                    }
                }
            }

            // Execute init statements from all modules
            for (_, module) in modules {
                let mut ctx = FuncEmitCtx::new(self, &init_locals);
                for stmt in &module.init {
                    ctx.emit_stmt(&mut func, stmt, false);
                }
            }

            func.instruction(&Instruction::End);
            code_section.function(&func);
        }

        wasm_module.section(&code_section);

        // --- Data section (string literal bytes, must come after Code) ---
        if !self.string_data.is_empty() {
            let mut data_section = DataSection::new();
            data_section.active(0, &wasm_encoder::ConstExpr::i32_const(0), self.string_data.iter().copied());
            wasm_module.section(&data_section);
        }

        wasm_module.finish()
    }

    fn compile_function(&self, hir_func: &perry_hir::ir::Function) -> Function {
        // Build local map: param locals come first, then body locals
        let mut local_map = BTreeMap::new();
        for (i, param) in hir_func.params.iter().enumerate() {
            local_map.insert(param.id, i as u32);
        }

        // Scan body for local variable declarations
        let param_count = hir_func.params.len() as u32;
        let mut extra_locals = 0u32;
        collect_locals(&hir_func.body, &mut local_map, &mut extra_locals, param_count);

        let locals = if extra_locals > 0 {
            vec![(extra_locals, ValType::F64)]
        } else {
            vec![]
        };
        let mut func = Function::new(locals);

        let has_ret = hir_func.body.iter().any(|s| has_return(s));
        let mut ctx = FuncEmitCtx::new(self, &local_map);

        for stmt in &hir_func.body {
            ctx.emit_stmt(&mut func, stmt, has_ret);
        }

        // If function should return but doesn't always, add a default return
        if has_ret {
            // Push undefined as default return
            func.instruction(&f64_const_bits(TAG_UNDEFINED));
        }

        func.instruction(&Instruction::End);
        func
    }

    fn collect_strings(&mut self, module: &perry_hir::ir::Module) {
        for func in &module.functions {
            self.collect_strings_in_stmts(&func.body);
        }
        for stmt in &module.init {
            self.collect_strings_in_stmt(stmt);
        }
        for global in &module.globals {
            if let Some(init) = &global.init {
                self.collect_strings_in_expr(init);
            }
        }
    }

    fn collect_strings_in_stmts(&mut self, stmts: &[Stmt]) {
        for stmt in stmts {
            self.collect_strings_in_stmt(stmt);
        }
    }

    fn collect_strings_in_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let { init, .. } => {
                if let Some(e) = init { self.collect_strings_in_expr(e); }
            }
            Stmt::Expr(e) => self.collect_strings_in_expr(e),
            Stmt::Return(e) => {
                if let Some(e) = e { self.collect_strings_in_expr(e); }
            }
            Stmt::If { condition, then_branch, else_branch } => {
                self.collect_strings_in_expr(condition);
                self.collect_strings_in_stmts(then_branch);
                if let Some(eb) = else_branch { self.collect_strings_in_stmts(eb); }
            }
            Stmt::While { condition, body } => {
                self.collect_strings_in_expr(condition);
                self.collect_strings_in_stmts(body);
            }
            Stmt::For { init, condition, update, body } => {
                if let Some(i) = init { self.collect_strings_in_stmt(i); }
                if let Some(c) = condition { self.collect_strings_in_expr(c); }
                if let Some(u) = update { self.collect_strings_in_expr(u); }
                self.collect_strings_in_stmts(body);
            }
            Stmt::Throw(e) => self.collect_strings_in_expr(e),
            Stmt::Try { body, catch, finally } => {
                self.collect_strings_in_stmts(body);
                if let Some(c) = catch {
                    self.collect_strings_in_stmts(&c.body);
                }
                if let Some(f) = finally { self.collect_strings_in_stmts(f); }
            }
            Stmt::Switch { discriminant, cases } => {
                self.collect_strings_in_expr(discriminant);
                for case in cases {
                    if let Some(t) = &case.test { self.collect_strings_in_expr(t); }
                    self.collect_strings_in_stmts(&case.body);
                }
            }
            Stmt::Break | Stmt::Continue => {}
        }
    }

    fn collect_strings_in_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::String(s) => { self.intern_string(s); }
            Expr::Binary { left, right, .. } | Expr::Compare { left, right, .. }
            | Expr::Logical { left, right, .. } => {
                self.collect_strings_in_expr(left);
                self.collect_strings_in_expr(right);
            }
            Expr::Unary { operand, .. } => self.collect_strings_in_expr(operand),
            Expr::Call { callee, args, .. } => {
                self.collect_strings_in_expr(callee);
                for a in args { self.collect_strings_in_expr(a); }
            }
            Expr::LocalSet(_, val) | Expr::GlobalSet(_, val) => {
                self.collect_strings_in_expr(val);
            }
            Expr::Conditional { condition, then_expr, else_expr } => {
                self.collect_strings_in_expr(condition);
                self.collect_strings_in_expr(then_expr);
                self.collect_strings_in_expr(else_expr);
            }
            Expr::Closure { body, .. } => {
                self.collect_strings_in_stmts(body);
            }
            Expr::NativeMethodCall { args, .. } => {
                for a in args { self.collect_strings_in_expr(a); }
            }
            Expr::Array(elems) => {
                for e in elems { self.collect_strings_in_expr(e); }
            }
            Expr::Object(fields) => {
                for (k, v) in fields {
                    self.intern_string(k);
                    self.collect_strings_in_expr(v);
                }
            }
            Expr::PropertyGet { object, property } => {
                self.collect_strings_in_expr(object);
                self.intern_string(property);
            }
            Expr::PropertySet { object, value, property, .. } => {
                self.collect_strings_in_expr(object);
                self.collect_strings_in_expr(value);
                self.intern_string(property);
            }
            Expr::IndexGet { object, index } => {
                self.collect_strings_in_expr(object);
                self.collect_strings_in_expr(index);
            }
            Expr::IndexSet { object, index, value } => {
                self.collect_strings_in_expr(object);
                self.collect_strings_in_expr(index);
                self.collect_strings_in_expr(value);
            }
            Expr::Await(e) | Expr::TypeOf(e) | Expr::Void(e) => {
                self.collect_strings_in_expr(e);
            }
            Expr::New { args, .. } => {
                for a in args { self.collect_strings_in_expr(a); }
            }
            Expr::Update { .. } | Expr::Sequence(_) => {}
            _ => {}
        }
    }
}

/// Context for emitting a single function body
struct FuncEmitCtx<'a> {
    emitter: &'a WasmModuleEmitter,
    local_map: &'a BTreeMap<LocalId, u32>,
    /// Block nesting depth for break/continue
    break_depth: Vec<u32>,
    loop_depth: Vec<u32>,
    block_depth: u32,
}

impl<'a> FuncEmitCtx<'a> {
    fn new(emitter: &'a WasmModuleEmitter, local_map: &'a BTreeMap<LocalId, u32>) -> Self {
        Self {
            emitter,
            local_map,
            break_depth: Vec::new(),
            loop_depth: Vec::new(),
            block_depth: 0,
        }
    }

    fn rt(&self) -> &RuntimeImports {
        self.emitter.rt.as_ref().unwrap()
    }

    fn emit_stmt(&mut self, func: &mut Function, stmt: &Stmt, in_returning_func: bool) {
        match stmt {
            Stmt::Let { id, init, .. } => {
                if let Some(init_expr) = init {
                    self.emit_expr(func, init_expr);
                } else {
                    // Default: undefined
                    func.instruction(&f64_const_bits(TAG_UNDEFINED));
                }
                if let Some(&idx) = self.local_map.get(id) {
                    func.instruction(&Instruction::LocalSet(idx));
                } else {
                    func.instruction(&Instruction::Drop);
                }
            }
            Stmt::Expr(expr) => {
                self.emit_expr(func, expr);
                // Drop the result (expression statement)
                // Check if expr produces a value
                if self.expr_has_value(expr) {
                    func.instruction(&Instruction::Drop);
                }
            }
            Stmt::Return(expr) => {
                if let Some(e) = expr {
                    self.emit_expr(func, e);
                } else if in_returning_func {
                    func.instruction(&f64_const_bits(TAG_UNDEFINED));
                }
                func.instruction(&Instruction::Return);
            }
            Stmt::If { condition, then_branch, else_branch } => {
                self.emit_expr(func, condition);
                // Convert to i32 boolean via is_truthy
                func.instruction(&Instruction::Call(self.rt().is_truthy));
                func.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                self.block_depth += 1;
                for s in then_branch {
                    self.emit_stmt(func, s, in_returning_func);
                }
                if let Some(else_stmts) = else_branch {
                    func.instruction(&Instruction::Else);
                    for s in else_stmts {
                        self.emit_stmt(func, s, in_returning_func);
                    }
                }
                self.block_depth -= 1;
                func.instruction(&Instruction::End);
            }
            Stmt::While { condition, body } => {
                // block $break
                //   loop $continue
                //     <condition>
                //     is_truthy
                //     i32.eqz
                //     br_if $break (1)
                //     <body>
                //     br $continue (0)
                //   end
                // end
                func.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                self.block_depth += 1;
                let break_depth = self.block_depth;
                self.break_depth.push(break_depth);

                func.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                self.block_depth += 1;
                let continue_depth = self.block_depth;
                self.loop_depth.push(continue_depth);

                self.emit_expr(func, condition);
                func.instruction(&Instruction::Call(self.rt().is_truthy));
                func.instruction(&Instruction::I32Eqz);
                func.instruction(&Instruction::BrIf(1)); // break to outer block

                for s in body {
                    self.emit_stmt(func, s, in_returning_func);
                }

                func.instruction(&Instruction::Br(0)); // continue (loop back)
                self.block_depth -= 1;
                func.instruction(&Instruction::End); // end loop

                self.loop_depth.pop();
                self.break_depth.pop();
                self.block_depth -= 1;
                func.instruction(&Instruction::End); // end block
            }
            Stmt::For { init, condition, update, body } => {
                // <init>
                // block $break
                //   loop $continue
                //     <condition>
                //     is_truthy ; i32.eqz ; br_if $break
                //     <body>
                //     <update> ; drop
                //     br $continue
                //   end
                // end
                if let Some(init_stmt) = init {
                    self.emit_stmt(func, init_stmt, in_returning_func);
                }

                func.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                self.block_depth += 1;
                self.break_depth.push(self.block_depth);

                func.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                self.block_depth += 1;
                self.loop_depth.push(self.block_depth);

                if let Some(cond) = condition {
                    self.emit_expr(func, cond);
                    func.instruction(&Instruction::Call(self.rt().is_truthy));
                    func.instruction(&Instruction::I32Eqz);
                    func.instruction(&Instruction::BrIf(1));
                }

                for s in body {
                    self.emit_stmt(func, s, in_returning_func);
                }

                if let Some(upd) = update {
                    self.emit_expr(func, upd);
                    if self.expr_has_value(upd) {
                        func.instruction(&Instruction::Drop);
                    }
                }

                func.instruction(&Instruction::Br(0));
                self.block_depth -= 1;
                func.instruction(&Instruction::End);

                self.loop_depth.pop();
                self.break_depth.pop();
                self.block_depth -= 1;
                func.instruction(&Instruction::End);
            }
            Stmt::Break => {
                // Branch to the enclosing block (break target)
                // The break target is 1 level up from the loop
                func.instruction(&Instruction::Br(1));
            }
            Stmt::Continue => {
                // Branch to the enclosing loop (continue target)
                func.instruction(&Instruction::Br(0));
            }
            Stmt::Throw(expr) => {
                // WASM doesn't have exceptions yet; just log and unreachable
                self.emit_expr(func, expr);
                func.instruction(&Instruction::Call(self.rt().console_error));
                func.instruction(&Instruction::Unreachable);
            }
            Stmt::Try { body, .. } => {
                // Best effort: just emit the try body (WASM exception handling is limited)
                for s in body {
                    self.emit_stmt(func, s, in_returning_func);
                }
            }
            Stmt::Switch { discriminant, cases } => {
                // Compile as chained if/else
                self.emit_expr(func, discriminant);
                // TODO: implement proper switch; for now emit as cascading checks
                func.instruction(&Instruction::Drop);
                for case in cases {
                    for s in &case.body {
                        self.emit_stmt(func, s, in_returning_func);
                    }
                }
            }
        }
    }

    fn emit_expr(&mut self, func: &mut Function, expr: &Expr) {
        match expr {
            // --- Literals ---
            Expr::Number(n) => {
                func.instruction(&f64_const(*n));
            }
            Expr::Integer(i) => {
                func.instruction(&f64_const(*i as f64));
            }
            Expr::Bool(true) => {
                func.instruction(&f64_const_bits(TAG_TRUE));
            }
            Expr::Bool(false) => {
                func.instruction(&f64_const_bits(TAG_FALSE));
            }
            Expr::Undefined => {
                func.instruction(&f64_const_bits(TAG_UNDEFINED));
            }
            Expr::Null => {
                func.instruction(&f64_const_bits(TAG_NULL));
            }
            Expr::String(s) => {
                let string_id = self.emitter.string_map.get(s.as_str())
                    .copied().unwrap_or(0);
                // NaN-box: (STRING_TAG << 48) | string_id
                let bits = (STRING_TAG << 48) | (string_id as u64);
                func.instruction(&f64_const(f64::from_bits(bits)));
            }

            // --- Variables ---
            Expr::LocalGet(id) => {
                if let Some(&idx) = self.local_map.get(id) {
                    func.instruction(&Instruction::LocalGet(idx));
                } else {
                    // Unknown local — push undefined
                    func.instruction(&f64_const_bits(TAG_UNDEFINED));
                }
            }
            Expr::LocalSet(id, val) => {
                self.emit_expr(func, val);
                if let Some(&idx) = self.local_map.get(id) {
                    // Tee: set and leave on stack
                    func.instruction(&Instruction::LocalTee(idx));
                }
            }
            Expr::GlobalGet(id) => {
                if let Some(&idx) = self.emitter.global_map.get(id) {
                    func.instruction(&Instruction::GlobalGet(idx));
                } else {
                    func.instruction(&f64_const_bits(TAG_UNDEFINED));
                }
            }
            Expr::GlobalSet(id, val) => {
                self.emit_expr(func, val);
                if let Some(&idx) = self.emitter.global_map.get(id) {
                    // Duplicate value on stack (set + leave result)
                    // WASM doesn't have GlobalTee, so we need a local
                    func.instruction(&Instruction::GlobalSet(idx));
                    func.instruction(&Instruction::GlobalGet(idx));
                }
            }

            // --- Update ---
            Expr::Update { id, op, prefix } => {
                if let Some(&idx) = self.local_map.get(id) {
                    if *prefix {
                        // ++x: increment then return new value
                        func.instruction(&Instruction::LocalGet(idx));
                        func.instruction(&f64_const(1.0));
                        match op {
                            UpdateOp::Increment => { func.instruction(&Instruction::F64Add); }
                            UpdateOp::Decrement => { func.instruction(&Instruction::F64Sub); }
                        };
                        func.instruction(&Instruction::LocalTee(idx));
                    } else {
                        // x++: return old value, then increment
                        func.instruction(&Instruction::LocalGet(idx));
                        // Compute new value
                        func.instruction(&Instruction::LocalGet(idx));
                        func.instruction(&f64_const(1.0));
                        match op {
                            UpdateOp::Increment => { func.instruction(&Instruction::F64Add); }
                            UpdateOp::Decrement => { func.instruction(&Instruction::F64Sub); }
                        };
                        func.instruction(&Instruction::LocalSet(idx));
                        // Old value is still on stack
                    }
                } else {
                    func.instruction(&f64_const(f64::NAN));
                }
            }

            // --- Binary operations ---
            Expr::Binary { op, left, right } => {
                match op {
                    BinaryOp::Add => {
                        // Use js_add for dynamic dispatch (handles string+number etc.)
                        self.emit_expr(func, left);
                        self.emit_expr(func, right);
                        func.instruction(&Instruction::Call(self.rt().js_add));
                    }
                    _ => {
                        // Pure numeric operations
                        self.emit_expr(func, left);
                        self.emit_expr(func, right);
                        match op {
                            BinaryOp::Sub => { func.instruction(&Instruction::F64Sub); }
                            BinaryOp::Mul => { func.instruction(&Instruction::F64Mul); }
                            BinaryOp::Div => { func.instruction(&Instruction::F64Div); }
                            BinaryOp::Mod => {
                                // WASM doesn't have f64.rem; approximate with div for now
                                func.instruction(&Instruction::F64Div);
                            }
                            BinaryOp::Pow => {
                                func.instruction(&Instruction::Call(self.rt().math_pow));
                            }
                            BinaryOp::BitAnd | BinaryOp::BitOr | BinaryOp::BitXor |
                            BinaryOp::Shl | BinaryOp::Shr | BinaryOp::UShr => {
                                // Bitwise ops: truncate f64 to i32, operate, convert back
                                // For now, just use f64.add as a placeholder
                                func.instruction(&Instruction::F64Add);
                            }
                            _ => { func.instruction(&Instruction::F64Add); }
                        };
                    }
                }
            }

            // --- Comparison ---
            Expr::Compare { op, left, right } => {
                self.emit_expr(func, left);
                self.emit_expr(func, right);
                // For strict equality on mixed types, use JS bridge
                match op {
                    CompareOp::Eq | CompareOp::Ne => {
                        func.instruction(&Instruction::Call(self.rt().js_strict_eq));
                        if matches!(op, CompareOp::Ne) {
                            func.instruction(&Instruction::I32Eqz);
                        }
                        // Convert i32 result to NaN-boxed boolean
                        func.instruction(&Instruction::If(wasm_encoder::BlockType::Result(ValType::F64)));
                        func.instruction(&f64_const_bits(TAG_TRUE));
                        func.instruction(&Instruction::Else);
                        func.instruction(&f64_const_bits(TAG_FALSE));
                        func.instruction(&Instruction::End);
                    }
                    _ => {
                        // Numeric comparisons
                        match op {
                            CompareOp::Lt => func.instruction(&Instruction::F64Lt),
                            CompareOp::Le => func.instruction(&Instruction::F64Le),
                            CompareOp::Gt => func.instruction(&Instruction::F64Gt),
                            CompareOp::Ge => func.instruction(&Instruction::F64Ge),
                            _ => unreachable!(),
                        };
                        // Convert i32 to NaN-boxed boolean
                        func.instruction(&Instruction::If(wasm_encoder::BlockType::Result(ValType::F64)));
                        func.instruction(&f64_const_bits(TAG_TRUE));
                        func.instruction(&Instruction::Else);
                        func.instruction(&f64_const_bits(TAG_FALSE));
                        func.instruction(&Instruction::End);
                    }
                }
            }

            // --- Logical ---
            Expr::Logical { op, left, right } => {
                match op {
                    LogicalOp::And => {
                        // Short-circuit: if left is falsy, return left; else return right
                        self.emit_expr(func, left);
                        func.instruction(&Instruction::Call(self.rt().is_truthy));
                        func.instruction(&Instruction::If(wasm_encoder::BlockType::Result(ValType::F64)));
                        self.emit_expr(func, right);
                        func.instruction(&Instruction::Else);
                        self.emit_expr(func, left);
                        func.instruction(&Instruction::End);
                    }
                    LogicalOp::Or => {
                        self.emit_expr(func, left);
                        func.instruction(&Instruction::Call(self.rt().is_truthy));
                        func.instruction(&Instruction::If(wasm_encoder::BlockType::Result(ValType::F64)));
                        self.emit_expr(func, left);
                        func.instruction(&Instruction::Else);
                        self.emit_expr(func, right);
                        func.instruction(&Instruction::End);
                    }
                    LogicalOp::Coalesce => {
                        // a ?? b: if a is null/undefined, return b
                        self.emit_expr(func, left);
                        // Check if null or undefined
                        // For simplicity, use is_truthy (close enough for first version)
                        func.instruction(&Instruction::Call(self.rt().is_truthy));
                        func.instruction(&Instruction::If(wasm_encoder::BlockType::Result(ValType::F64)));
                        self.emit_expr(func, left);
                        func.instruction(&Instruction::Else);
                        self.emit_expr(func, right);
                        func.instruction(&Instruction::End);
                    }
                }
            }

            // --- Unary ---
            Expr::Unary { op, operand } => {
                self.emit_expr(func, operand);
                match op {
                    UnaryOp::Neg => { func.instruction(&Instruction::F64Neg); }
                    UnaryOp::Pos => {} // no-op for numbers
                    UnaryOp::Not => {
                        func.instruction(&Instruction::Call(self.rt().is_truthy));
                        func.instruction(&Instruction::I32Eqz);
                        func.instruction(&Instruction::If(wasm_encoder::BlockType::Result(ValType::F64)));
                        func.instruction(&f64_const_bits(TAG_TRUE));
                        func.instruction(&Instruction::Else);
                        func.instruction(&f64_const_bits(TAG_FALSE));
                        func.instruction(&Instruction::End);
                    }
                    UnaryOp::BitNot => {
                        // ~x: convert to i32, bitwise not, convert back
                        func.instruction(&Instruction::I32TruncF64S);
                        func.instruction(&Instruction::I32Const(-1));
                        func.instruction(&Instruction::I32Xor);
                        func.instruction(&Instruction::F64ConvertI32S);
                    }
                };
            }

            // --- Function calls ---
            Expr::Call { callee, args, .. } => {
                // Check for console.log/warn/error pattern:
                // Call { callee: PropertyGet { object: GlobalGet(0), property: "log" }, args }
                if let Expr::PropertyGet { object, property } = callee.as_ref() {
                    if let Expr::GlobalGet(_) = object.as_ref() {
                        match property.as_str() {
                            "log" => {
                                for arg in args {
                                    self.emit_expr(func, arg);
                                    func.instruction(&Instruction::Call(self.rt().console_log));
                                }
                                return;
                            }
                            "warn" => {
                                for arg in args {
                                    self.emit_expr(func, arg);
                                    func.instruction(&Instruction::Call(self.rt().console_warn));
                                }
                                return;
                            }
                            "error" => {
                                for arg in args {
                                    self.emit_expr(func, arg);
                                    func.instruction(&Instruction::Call(self.rt().console_error));
                                }
                                return;
                            }
                            _ => {}
                        }
                    }
                }

                // Evaluate arguments first
                for arg in args {
                    self.emit_expr(func, arg);
                }
                // Call the function
                match callee.as_ref() {
                    Expr::FuncRef(id) => {
                        if let Some(&idx) = self.emitter.func_map.get(id) {
                            func.instruction(&Instruction::Call(idx));
                        } else {
                            // Unknown function — push undefined
                            for _ in args {
                                func.instruction(&Instruction::Drop);
                            }
                            func.instruction(&f64_const_bits(TAG_UNDEFINED));
                        }
                    }
                    _ => {
                        // Dynamic call — can't handle in WASM without tables
                        for _ in args {
                            func.instruction(&Instruction::Drop);
                        }
                        func.instruction(&f64_const_bits(TAG_UNDEFINED));
                    }
                }
            }

            Expr::FuncRef(_) => {
                // Function reference as a value — not yet supported in WASM mode
                func.instruction(&f64_const_bits(TAG_UNDEFINED));
            }

            // --- Native method calls (console.log, etc.) ---
            Expr::NativeMethodCall { module, method, args, .. } => {
                let normalized = module.strip_prefix("node:").unwrap_or(module);
                match normalized {
                    "console" => {
                        match method.as_str() {
                            "log" => {
                                for arg in args {
                                    self.emit_expr(func, arg);
                                    func.instruction(&Instruction::Call(self.rt().console_log));
                                }
                            }
                            "warn" => {
                                for arg in args {
                                    self.emit_expr(func, arg);
                                    func.instruction(&Instruction::Call(self.rt().console_warn));
                                }
                            }
                            "error" => {
                                for arg in args {
                                    self.emit_expr(func, arg);
                                    func.instruction(&Instruction::Call(self.rt().console_error));
                                }
                            }
                            _ => {
                                // Unsupported console method
                            }
                        }
                    }
                    _ => {
                        // Unsupported native module — no-op
                    }
                }
            }

            // --- Conditional (ternary) ---
            Expr::Conditional { condition, then_expr, else_expr } => {
                self.emit_expr(func, condition);
                func.instruction(&Instruction::Call(self.rt().is_truthy));
                func.instruction(&Instruction::If(wasm_encoder::BlockType::Result(ValType::F64)));
                self.emit_expr(func, then_expr);
                func.instruction(&Instruction::Else);
                self.emit_expr(func, else_expr);
                func.instruction(&Instruction::End);
            }

            // --- Math ---
            Expr::MathFloor(x) => {
                self.emit_expr(func, x);
                func.instruction(&Instruction::F64Floor);
            }
            Expr::MathCeil(x) => {
                self.emit_expr(func, x);
                func.instruction(&Instruction::F64Ceil);
            }
            Expr::MathAbs(x) => {
                self.emit_expr(func, x);
                func.instruction(&Instruction::F64Abs);
            }
            Expr::MathSqrt(x) => {
                self.emit_expr(func, x);
                func.instruction(&Instruction::F64Sqrt);
            }
            Expr::MathRound(x) => {
                self.emit_expr(func, x);
                func.instruction(&Instruction::F64Nearest);
            }
            Expr::MathPow(base, exp) => {
                self.emit_expr(func, base);
                self.emit_expr(func, exp);
                func.instruction(&Instruction::Call(self.rt().math_pow));
            }
            Expr::MathMin(args) if args.len() == 2 => {
                self.emit_expr(func, &args[0]);
                self.emit_expr(func, &args[1]);
                func.instruction(&Instruction::F64Min);
            }
            Expr::MathMax(args) if args.len() == 2 => {
                self.emit_expr(func, &args[0]);
                self.emit_expr(func, &args[1]);
                func.instruction(&Instruction::F64Max);
            }
            Expr::MathRandom => {
                func.instruction(&Instruction::Call(self.rt().math_random));
            }

            // --- Typeof ---
            Expr::TypeOf(operand) => {
                self.emit_expr(func, operand);
                func.instruction(&Instruction::Call(self.rt().js_typeof));
            }

            // --- Misc expressions that produce undefined for now ---
            Expr::Await(e) => {
                // WASM is synchronous; just evaluate the expression
                self.emit_expr(func, e);
            }

            Expr::ExternFuncRef { .. } | Expr::ClassRef(_) | Expr::This |
            Expr::SuperCall(_) | Expr::New { .. } | Expr::Array(_) | Expr::Object(_) |
            Expr::PropertyGet { .. } | Expr::PropertySet { .. } |
            Expr::IndexGet { .. } | Expr::IndexSet { .. } |
            Expr::InstanceOf { .. } | Expr::In { .. } | Expr::Delete(_) |
            Expr::Closure { .. } | Expr::Void(_) => {
                func.instruction(&f64_const_bits(TAG_UNDEFINED));
            }

            // --- DateNow ---
            Expr::DateNow => {
                func.instruction(&Instruction::Call(self.rt().date_now));
            }

            // --- Sequence ---
            Expr::Sequence(exprs) => {
                for (i, e) in exprs.iter().enumerate() {
                    self.emit_expr(func, e);
                    if i < exprs.len() - 1 {
                        func.instruction(&Instruction::Drop);
                    }
                }
                if exprs.is_empty() {
                    func.instruction(&f64_const_bits(TAG_UNDEFINED));
                }
            }

            // --- Catch-all: emit undefined ---
            _ => {
                func.instruction(&f64_const_bits(TAG_UNDEFINED));
            }
        }
    }

    /// Check if an expression produces a value on the stack
    fn expr_has_value(&self, expr: &Expr) -> bool {
        match expr {
            Expr::NativeMethodCall { module, .. } => {
                let normalized = module.strip_prefix("node:").unwrap_or(module);
                if normalized == "console" {
                    return false;
                }
                true
            }
            // console.log/warn/error via Call + PropertyGet pattern
            Expr::Call { callee, .. } => {
                if let Expr::PropertyGet { object, property } = callee.as_ref() {
                    if let Expr::GlobalGet(_) = object.as_ref() {
                        if matches!(property.as_str(), "log" | "warn" | "error") {
                            return false;
                        }
                    }
                }
                true
            }
            _ => true,
        }
    }
}

/// Recursively scan statements for local variable declarations
fn collect_locals(stmts: &[Stmt], map: &mut BTreeMap<LocalId, u32>, count: &mut u32, offset: u32) {
    for stmt in stmts {
        match stmt {
            Stmt::Let { id, .. } => {
                if !map.contains_key(id) {
                    map.insert(*id, offset + *count);
                    *count += 1;
                }
            }
            Stmt::If { then_branch, else_branch, .. } => {
                collect_locals(then_branch, map, count, offset);
                if let Some(eb) = else_branch {
                    collect_locals(eb, map, count, offset);
                }
            }
            Stmt::While { body, .. } | Stmt::For { body, .. } => {
                collect_locals(body, map, count, offset);
            }
            Stmt::Try { body, catch, finally } => {
                collect_locals(body, map, count, offset);
                if let Some(c) = catch {
                    if let Some((id, _)) = &c.param {
                        if !map.contains_key(id) {
                            map.insert(*id, offset + *count);
                            *count += 1;
                        }
                    }
                    collect_locals(&c.body, map, count, offset);
                }
                if let Some(f) = finally {
                    collect_locals(f, map, count, offset);
                }
            }
            Stmt::Switch { cases, .. } => {
                for case in cases {
                    collect_locals(&case.body, map, count, offset);
                }
            }
            _ => {}
        }
    }
}

/// Check if a statement or its children contain a return statement
fn has_return(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Return(_) => true,
        Stmt::If { then_branch, else_branch, .. } => {
            then_branch.iter().any(has_return) ||
            else_branch.as_ref().map_or(false, |eb| eb.iter().any(has_return))
        }
        Stmt::While { body, .. } | Stmt::For { body, .. } => {
            body.iter().any(has_return)
        }
        Stmt::Try { body, catch, finally } => {
            body.iter().any(has_return) ||
            catch.as_ref().map_or(false, |c| c.body.iter().any(has_return)) ||
            finally.as_ref().map_or(false, |f| f.iter().any(has_return))
        }
        _ => false,
    }
}
