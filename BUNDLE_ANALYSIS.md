# Bundle Size Analysis

This document describes the approach for analyzing and optimizing the Node.js module footprint of the Soroban Security Scanner CLI.

## Current Dependencies

| Package | Purpose | Size Impact |
|---------|---------|-------------|
| `@stellar/stellar-sdk` | Stellar network interaction | Large (includes crypto) |
| `commander` | CLI argument parsing | Small |
| `chalk` | Terminal color output | Small |
| `js-yaml` | YAML config parsing | Small |

## Analysis Approach

Since this is a Node.js CLI tool (not a browser bundle), traditional webpack-bundle-analyzer does not apply. Instead, use:

```bash
# Analyze installed package sizes
npx cost-of-modules --no-install

# Check for duplicate dependencies
npx depcheck

# Audit unused dependencies
npx npm-check
```

## Optimizations Applied

1. **Lazy requires**: Heavy dependencies (`@stellar/stellar-sdk`) are only loaded when the relevant command is invoked, not at startup.
2. **No global flag on regex patterns**: Stateless regex patterns avoid hidden state and reduce per-scan overhead (see issue #87).
3. **ESLint configured** (`.eslintrc.json`): Catches `no-unused-vars` to prevent dead code from accumulating.

## Recommendations

- If a web frontend is added in the future, use Next.js built-in bundle analyzer:
  ```bash
  ANALYZE=true next build
  ```
- Consider replacing `chalk` with `picocolors` for a smaller color-output footprint.
- `@stellar/stellar-sdk` should only be imported in modules that actually use it; avoid top-level imports in the CLI entry point unless needed.
