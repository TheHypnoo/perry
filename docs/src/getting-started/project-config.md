# Project Configuration

Perry projects use standard `package.json` for configuration. No special config file is required for basic usage, but larger projects benefit from Perry-specific settings.

## Basic Setup

```bash
perry init my-project
cd my-project
```

This creates a `package.json` and a starter `src/index.ts`.

## package.json

```json
{
  "name": "my-project",
  "version": "1.0.0",
  "main": "src/index.ts",
  "perry": {
    "compilePackages": []
  }
}
```

### Perry Configuration

The `perry` field in `package.json` controls compiler behavior:

#### `compilePackages`

List npm packages to compile natively instead of routing through the JavaScript runtime:

```json
{
  "perry": {
    "compilePackages": ["@noble/curves", "@noble/hashes"]
  }
}
```

When a package is listed here, Perry:
1. Resolves the package in `node_modules/`
2. Prefers TypeScript source (`src/index.ts`) over compiled JavaScript (`lib/index.js`)
3. Compiles all functions natively through Cranelift
4. Deduplicates across nested `node_modules/` to prevent duplicate linker symbols

This is useful for pure TypeScript/JavaScript packages that don't rely on Node.js APIs. Packages that use native bindings, `eval()`, or dynamic `require()` won't work.

## Using npm Packages

Perry natively supports many popular npm packages without any configuration:

```typescript
import fastify from "fastify";
import mysql from "mysql2/promise";
import Redis from "ioredis";
import bcrypt from "bcrypt";
```

These are compiled to native code using Perry's built-in implementations. See [Standard Library](../stdlib/overview.md) for the full list.

For packages not natively supported, use `compilePackages` for pure TS/JS packages, or the JavaScript runtime fallback for complex packages.

## Project Structure

Perry is flexible about project structure. Common patterns:

```
my-project/
├── package.json
├── src/
│   └── index.ts
└── node_modules/      # Only needed for compilePackages
```

For UI apps:

```
my-app/
├── package.json
├── src/
│   ├── index.ts       # Main app entry
│   └── components/    # UI components
└── assets/            # Images, etc.
```

## Compilation

```bash
# Compile a file
perry src/index.ts -o build/app

# Compile with a specific target
perry src/index.ts -o build/app --target ios-simulator

# Debug: print intermediate representation
perry src/index.ts --print-hir
```

See [CLI Commands](../cli/commands.md) for all options.

## Next Steps

- [CLI Commands](../cli/commands.md) — All compiler commands and flags
- [Supported Features](../language/supported-features.md) — What TypeScript features work
- [Standard Library](../stdlib/overview.md) — Supported npm packages
