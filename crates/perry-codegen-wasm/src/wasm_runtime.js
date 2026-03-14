// Perry WASM Runtime Bridge
// Provides JavaScript runtime functions imported by the WASM module.
// Handles NaN-boxing, string management, and browser API access.

// NaN-boxing constants (matching perry-runtime/src/value.rs)
const TAG_UNDEFINED = 0x7FFC_0000_0000_0001n;
const TAG_NULL      = 0x7FFC_0000_0000_0002n;
const TAG_FALSE     = 0x7FFC_0000_0000_0003n;
const TAG_TRUE      = 0x7FFC_0000_0000_0004n;
const STRING_TAG    = 0x7FFFn;
const POINTER_TAG   = 0x7FFDn;
const INT32_TAG     = 0x7FFEn;

// f64 <-> i64 conversion via shared buffer
const _convBuf = new ArrayBuffer(8);
const _f64 = new Float64Array(_convBuf);
const _u64 = new BigUint64Array(_convBuf);

function f64ToU64(f) { _f64[0] = f; return _u64[0]; }
function u64ToF64(u) { _u64[0] = u; return _f64[0]; }

function nanboxString(id) {
  return u64ToF64((STRING_TAG << 48n) | BigInt(id));
}

function isString(val) {
  return (f64ToU64(val) >> 48n) === STRING_TAG;
}

function getStringId(val) {
  return Number(f64ToU64(val) & 0xFFFFFFFFn);
}

function isUndefined(val) { return f64ToU64(val) === TAG_UNDEFINED; }
function isNull(val) { return f64ToU64(val) === TAG_NULL; }
function isTrue(val) { return f64ToU64(val) === TAG_TRUE; }
function isFalse(val) { return f64ToU64(val) === TAG_FALSE; }
function isBool(val) { const b = f64ToU64(val); return b === TAG_TRUE || b === TAG_FALSE; }

// String table — maps string_id (index) to JS string
const stringTable = [];

// Convert a NaN-boxed f64 value to a JS value for display/manipulation
function toJsValue(val) {
  const bits = f64ToU64(val);
  if (bits === TAG_UNDEFINED) return undefined;
  if (bits === TAG_NULL) return null;
  if (bits === TAG_TRUE) return true;
  if (bits === TAG_FALSE) return false;
  const tag = bits >> 48n;
  if (tag === STRING_TAG) return stringTable[Number(bits & 0xFFFFFFFFn)];
  // Plain number
  return val;
}

// Convert a JS value to a NaN-boxed f64
function fromJsValue(v) {
  if (v === undefined) return u64ToF64(TAG_UNDEFINED);
  if (v === null) return u64ToF64(TAG_NULL);
  if (v === true) return u64ToF64(TAG_TRUE);
  if (v === false) return u64ToF64(TAG_FALSE);
  if (typeof v === 'string') {
    const id = stringTable.length;
    stringTable.push(v);
    return nanboxString(id);
  }
  return v; // number
}

let wasmMemory = null;

// Build the import object for WASM instantiation
function buildImports() {
  return {
    rt: {
      // Register a string literal from WASM memory
      string_new: (offset, len) => {
        const bytes = new Uint8Array(wasmMemory.buffer, offset, len);
        const str = new TextDecoder().decode(bytes);
        stringTable.push(str);
      },

      // Console output
      console_log: (val) => {
        console.log(toJsValue(val));
      },
      console_warn: (val) => {
        console.warn(toJsValue(val));
      },
      console_error: (val) => {
        console.error(toJsValue(val));
      },

      // String concatenation: string + string -> string
      string_concat: (a, b) => {
        const sa = stringTable[getStringId(a)];
        const sb = stringTable[getStringId(b)];
        const id = stringTable.length;
        stringTable.push(sa + sb);
        return nanboxString(id);
      },

      // Dynamic addition: handles string+string, string+number, number+string
      js_add: (a, b) => {
        const ja = toJsValue(a);
        const jb = toJsValue(b);
        const result = ja + jb;
        return fromJsValue(result);
      },

      // String comparison
      string_eq: (a, b) => {
        const sa = stringTable[getStringId(a)];
        const sb = stringTable[getStringId(b)];
        return sa === sb ? 1 : 0;
      },

      // String length
      string_len: (val) => {
        return stringTable[getStringId(val)].length;
      },

      // Convert any value to string
      jsvalue_to_string: (val) => {
        const js = toJsValue(val);
        const str = String(js);
        const id = stringTable.length;
        stringTable.push(str);
        return nanboxString(id);
      },

      // Check if a value is truthy (returns i32: 0 or 1)
      is_truthy: (val) => {
        const bits = f64ToU64(val);
        if (bits === TAG_FALSE || bits === TAG_NULL || bits === TAG_UNDEFINED) return 0;
        if (bits === TAG_TRUE) return 1;
        const tag = bits >> 48n;
        if (tag === STRING_TAG) {
          return stringTable[Number(bits & 0xFFFFFFFFn)].length > 0 ? 1 : 0;
        }
        // Number: 0 and NaN are falsy
        return (val === 0 || Number.isNaN(val)) ? 0 : 1;
      },

      // Strict equality
      js_strict_eq: (a, b) => {
        const ja = toJsValue(a);
        const jb = toJsValue(b);
        return ja === jb ? 1 : 0;
      },

      // String methods
      string_charAt: (str, idx) => {
        const s = stringTable[getStringId(str)];
        const ch = s.charAt(idx);
        const id = stringTable.length;
        stringTable.push(ch);
        return nanboxString(id);
      },
      string_substring: (str, start, end) => {
        const s = stringTable[getStringId(str)];
        const result = s.substring(start, end);
        const id = stringTable.length;
        stringTable.push(result);
        return nanboxString(id);
      },
      string_indexOf: (str, search) => {
        const s = stringTable[getStringId(str)];
        const needle = stringTable[getStringId(search)];
        return s.indexOf(needle);
      },
      string_slice: (str, start, end) => {
        const s = stringTable[getStringId(str)];
        const result = s.slice(start, end);
        const id = stringTable.length;
        stringTable.push(result);
        return nanboxString(id);
      },
      string_toLowerCase: (str) => {
        const s = stringTable[getStringId(str)];
        const id = stringTable.length;
        stringTable.push(s.toLowerCase());
        return nanboxString(id);
      },
      string_toUpperCase: (str) => {
        const s = stringTable[getStringId(str)];
        const id = stringTable.length;
        stringTable.push(s.toUpperCase());
        return nanboxString(id);
      },
      string_trim: (str) => {
        const s = stringTable[getStringId(str)];
        const id = stringTable.length;
        stringTable.push(s.trim());
        return nanboxString(id);
      },
      string_includes: (str, search) => {
        const s = stringTable[getStringId(str)];
        const needle = stringTable[getStringId(search)];
        return s.includes(needle) ? 1 : 0;
      },
      string_startsWith: (str, search) => {
        const s = stringTable[getStringId(str)];
        const needle = stringTable[getStringId(search)];
        return s.startsWith(needle) ? 1 : 0;
      },
      string_endsWith: (str, search) => {
        const s = stringTable[getStringId(str)];
        const needle = stringTable[getStringId(search)];
        return s.endsWith(needle) ? 1 : 0;
      },
      string_replace: (str, pattern, replacement) => {
        const s = stringTable[getStringId(str)];
        const p = stringTable[getStringId(pattern)];
        const r = stringTable[getStringId(replacement)];
        const id = stringTable.length;
        stringTable.push(s.replace(p, r));
        return nanboxString(id);
      },
      string_split: (str, delim) => {
        // Returns a comma-joined string for now (arrays need more work)
        const s = stringTable[getStringId(str)];
        const d = stringTable[getStringId(delim)];
        const id = stringTable.length;
        stringTable.push(JSON.stringify(s.split(d)));
        return nanboxString(id);
      },

      // Math
      math_floor: (x) => Math.floor(x),
      math_ceil: (x) => Math.ceil(x),
      math_round: (x) => Math.round(x),
      math_abs: (x) => Math.abs(x),
      math_sqrt: (x) => Math.sqrt(x),
      math_pow: (base, exp) => Math.pow(base, exp),
      math_min: (a, b) => Math.min(a, b),
      math_max: (a, b) => Math.max(a, b),
      math_random: () => Math.random(),
      math_log: (x) => Math.log(x),
      math_log2: (x) => Math.log2(x),
      math_log10: (x) => Math.log10(x),

      // parseInt / parseFloat
      parse_int: (str) => {
        const s = stringTable[getStringId(str)];
        return parseInt(s, 10);
      },
      parse_float: (str) => {
        const s = stringTable[getStringId(str)];
        return parseFloat(s);
      },

      // Date
      date_now: () => Date.now(),

      // typeof (returns string)
      js_typeof: (val) => {
        const js = toJsValue(val);
        const t = typeof js;
        const id = stringTable.length;
        stringTable.push(t);
        return nanboxString(id);
      },
    }
  };
}

// Boot the WASM module
async function bootPerryWasm(wasmBase64) {
  const wasmBytes = Uint8Array.from(atob(wasmBase64), c => c.charCodeAt(0));
  const imports = buildImports();
  const { instance } = await WebAssembly.instantiate(wasmBytes, imports);
  wasmMemory = instance.exports.memory;
  // Call the entry point
  if (instance.exports._start) {
    instance.exports._start();
  } else if (instance.exports.main) {
    instance.exports.main();
  }
}
