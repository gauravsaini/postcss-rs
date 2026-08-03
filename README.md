# PostCSS Rust Parser (`postcss-rs`)

High-performance Rust-based parser for CSS, compiled to a native Node.js addon using NAPI-RS. This package serves as a drop-in replacement parser for the standard JavaScript-based PostCSS parsing engine.

## 🚀 Architecture & High-Level Design (HLD)

The traditional method of interfacing between Rust parsers and JavaScript is to parse CSS into tree structures in Rust, serialize the entire tree to JSON, and deserialize it on the Node.js side. However, the CPU serialization/deserialization overhead on large CSS files can completely negate any parsing speed gains.

To resolve this bottleneck, `postcss-rs` utilizes a **Flat Shared-Memory Buffer** design:

1. **Rust parsing**: The Rust tokenizer and parser processes the CSS string and constructs raw CSS node boundaries and settings.
2. **Buffer Serialization**: Instead of sending a deep nested object structure, Rust serializes all node structural information into a single 1D flat integer array (`Int32Array` called `metadata`).
3. **Strings Packing**: All dynamic strings (such as selectors, property names, rule/decl values, spaces, and raws formatting tokens) are concatenated into one massive single string (`bigString`).
4. **NAPI Boundary Transfer**: Node.js receives the flat metadata buffer (transferred zero-copy via NAPI `ArrayBuffer`) and the packed string (copied across the NAPI boundary as a JS string). The metadata buffer — which constitutes the structural backbone of the AST — avoids serialization/deserialization overhead entirely. The string data requires a single bulk copy but avoids per-node string allocation. The JS bridge then reconstructs the standard PostCSS JS AST classes (`Root`, `Rule`, `Declaration`, `AtRule`, `Comment`) by simple array index offsets and substring extraction.

```
                   ┌──────────────────┐
                   │    CSS Source    │
                   └────────┬─────────┘
                            │
                            ▼
                   ┌──────────────────┐
                   │  Rust Tokenizer  │
                   └────────┬─────────┘
                            │
                            ▼
                   ┌──────────────────┐
                   │   Rust Parser    │
                   └────────┬─────────┘
                            │
                            ▼
                   ┌──────────────────┐
                   │Packed AstBuffer  │
                   │ (metadata, str)  │
                   └────────┬─────────┘
                            │  [ NAPI Zero-Copy Transfer ]
                            ▼
                   ┌──────────────────┐
                   │   JS bridge.js   │
                   └────────┬─────────┘
                            │  [ Fast AST Reconstruction ]
                            ▼
                   ┌──────────────────┐
                   │  PostCSS AST JS  │
                   └──────────────────┘
```

## 🛠️ Low-Level Design (LLD)

### Metadata Schema
Each parsed CSS node occupies exactly **23 contiguous `i32` slots** in the `metadata` array:
- **Slot 0**: `node_type` (0: Root, 1: Rule, 2: Declaration, 3: AtRule, 4: Comment)
- **Slot 1**: `parent_id` (Index of the parent node inside the metadata array, or `-1` for Root)
- **Slot 2-4**: Start location coordinates (`offset`, `line`, `column`)
- **Slot 5-7**: End location coordinates (`offset`, `line`, `column`)
- **Slot 8**: `important` (Boolean flag, `1` if the declaration has `!important`, else `0`)
- **Slot 9**: `semicolon` (Boolean flag indicating whether the block has a trailing semicolon)
- **Slot 10**: `has_nodes` (Boolean flag indicating child node presence)
- **Slot 11-22**: Dynamic attribute descriptor pairs (6 pairs of `[offset, length]` into `bigString`):
  - **Type 0 (Root)**: Slots for `after` spacing.
  - **Type 1 (Rule)**: Slots for `selector`, `ownSemicolon`, `before`, `between`, `after`, and `selector_raw`.
  - **Type 2 (Decl)**: Slots for `prop`, `value`, `before`, `between`, `value_raw`, and `important_raw`.
  - **Type 3 (AtRule)**: Slots for `name`, `params`, `before`, `between`, `after`, and `afterName`.
  - **Type 4 (Comment)**: Slots for `text`, `unused`, `before`, `left`, and `right` space.

## ⚡ Performance & Benchmark Results

Benchmark results measured across 50 iterations with `--expose-gc` (median execution time):

### Synthetic Large Document (1.63 MB / ~64,500 lines)

| Parser Engine / Mode | Execution Time (ms) | Speedup vs JS PostCSS |
| :--- | :---: | :---: |
| **Original PostCSS (Pure JS)** | ~49.3 ms | 1.0× (Baseline) |
| **Rust Parse + Buffer Only** _(not a usable AST)_ | ~16.6 ms | **~2.97× faster** ⚡ |
| **Drop-in Replacement (`postcss-rs`)** | ~29.3 ms | **~1.68× faster** 🚀 |

### Real-World CSS Fixtures (~66 KB / ~1,300–1,500 lines)

| Fixture Input | JS PostCSS | Drop-in (`postcss-rs`) | Drop-in Speedup | Buffer Only | Buffer Speedup |
| :--- | :---: | :---: | :---: | :---: | :---: |
| **Bootstrap-like CSS** (65.8 KB) | 1.52 ms | 1.20 ms | **1.27× faster** | 0.78 ms | 1.94× faster |
| **Tailwind-like CSS** (66.7 KB) | 1.66 ms | 1.35 ms | **1.23× faster** | 0.87 ms | 1.91× faster |

> **Note:** "Rust Parse + Buffer Only" measures Rust parsing and buffer serialization without JS AST reconstruction. It returns a raw `Int32Array` metadata buffer, not a usable PostCSS AST. The "Drop-in Replacement" is the relevant comparison for end-users — it returns a fully usable PostCSS AST with verified structural parity. For smaller files (~66 KB), NAPI boundary and JS AST reconstruction overhead is a larger fraction of runtime, resulting in ~1.25× speedups, while larger CSS files demonstrate ~1.68× speedups.

### Key Optimizations
- **`mimalloc` Allocator Integration**: Configured Microsoft's `mimalloc` as global Rust allocator (`#[global_allocator]`), minimizing native heap allocation latency.
- **$\mathcal{O}(1)$ Lookup Table (LUT) Tokenization**: Static 256-byte character classification tables (`IS_WORD_STOP`, `IS_AT_STOP`) eliminate branching overhead during word boundary detection.
- **Fast Prototype Re-hydration**: JS bridge uses `Object.create(Prototype)` fast instantiation to bypass JS constructor overhead when materializing 50,000+ AST nodes.

## 📦 Getting Started

### Prerequisites
- Node.js (v18+)
- Rust toolchain (cargo, rustc)

### Installation
Clone the repository and install dependencies:
```bash
pnpm install
```

### Building the Native Addon
To build the optimized release target:
```bash
pnpm run build
```

To build a debug version:
```bash
pnpm run build:debug
```

### Running Benchmarks
Run the benchmark suite comparing JS PostCSS vs `postcss-rs` (use `--expose-gc` for best results):
```bash
node --expose-gc benchmark.js
```

## 🧪 Testing and Verification
Run the comprehensive test suite (AST parity, real-world fixture verification, and fuzz testing):
```bash
pnpm test
```

Run fuzz testing separately with a custom seed for reproducibility:
```bash
FUZZ_SEED=42 FUZZ_ITERATIONS=1000 node test_fuzz.js
```

## 🔌 PostCSS Plugin Ecosystem Compatibility

`@gauravsaini/postcss-rs` is 100% compatible out-of-the-box with the entire PostCSS plugin ecosystem.

Because `@gauravsaini/postcss-rs` rehydrates its Rust memory buffer directly into standard PostCSS AST class instances (`Root`, `Rule`, `Declaration`, `AtRule`, `Comment`), plugins inherit all standard AST traversal and mutation methods seamlessly (`walkRules`, `walkDecls`, `append`, `replaceWith`, `clone`, `toString`, etc.).

### Supported Plugin Categories out-of-the-box:
- **Future CSS & Prefixes**: `autoprefixer`, `postcss-preset-env`
- **Component & Selector Isolation**: `postcss-modules`, `react-css-modules`, `postcss-initial`, `postcss-autoreset`, `cq-prolyfill`
- **Preprocessing & Nesting**: `postcss-nested`, `postcss-sorting`, `postcss-utilities`, `short`
- **Assets & Media**: `postcss-url`, `postcss-sprites`, `font-magician`, `postcss-inline-svg`, `webp-in-css`, `avif-in-css`
- **Linters & Diagnostics**: `stylelint`, `doiuse`, `colorguard`
- **Minifiers & Language Utilities**: `cssnano`, `lost`, `rtlcss`

### Using with PostCSS Plugins

```javascript
const postcss = require('postcss');
const postcssRs = require('@gauravsaini/postcss-rs');
const autoprefixer = require('autoprefixer');
const postcssNested = require('postcss-nested');
const cssnano = require('cssnano');

const css = `
.card {
  display: flex;
  user-select: none;
  & .title { font-size: 1.2rem; }
}
`;

// Pass postcssRs.parse as the parser option to PostCSS
postcss([postcssNested(), autoprefixer(), cssnano()])
  .process(css, { parser: postcssRs.parse, from: 'input.css' })
  .then(result => {
    console.log(result.css);
  });
```

## 🤖 CI/CD & npm Publishing

Automated testing, cross-compilation, and npm releases are configured via GitHub Actions:

- **Continuous Integration (`.github/workflows/ci.yml`)**: Runs linting, native build, and the full test suite across macOS, Linux, and Windows on every PR and main branch push.
- **Automated Publishing (`.github/workflows/release.yml`)**: Triggered when a new git tag (e.g. `v1.0.0`) is pushed or manually via `workflow_dispatch`.
  - Matrix builds native `.node` binaries for 7 target architectures (macOS x64/arm64, Linux x64/arm64 gnu/musl, Windows x64/arm64).
  - Downloads built binaries, runs verification tests against all target binaries, and publishes the package to npm using `NPM_TOKEN`.
  - Automatically creates a GitHub Release with attached native binary artifacts.

### Releasing a New Version
1. Ensure your working directory is clean and tests pass (`pnpm test`).
2. Tag and push a release:
   ```bash
   git tag v1.0.1
   git push origin v1.0.1
   ```
3. GitHub Actions will build native binaries for all platforms and publish to npm automatically.
