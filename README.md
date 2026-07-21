# PostCSS Rust Parser (`postcss-rs`)

High-performance Rust-based parser for CSS, compiled to a native Node.js addon using NAPI-RS. This package serves as a drop-in replacement parser for the standard JavaScript-based PostCSS parsing engine.

## 🚀 Architecture & High-Level Design (HLD)

The traditional method of interfacing between Rust parsers and JavaScript is to parse CSS into a tree structures in Rust, serialize the entire tree to JSON, and deserialize it on the Node.js side. However, the CPU serialization/deserialization overhead on large CSS files can completely negate any parsing speed gains.

To resolve this bottleneck, `postcss-rs` utilizes a **Flat Shared-Memory Buffer** design:

1. **Rust parsing**: The Rust tokenizer and parser processes the CSS string and constructs raw CSS node boundaries and settings.
2. **Buffer Serialization**: Instead of sending a deep nested object structure, Rust serializes all node structural information into a single 1D flat integer array (`Int32Array` called `metadata`).
3. **Strings Packing**: All dynamic strings (such as selectors, property names, rule/decl values, spaces, and raws formatting tokens) are concatenated into one massive single string (`bigString`).
4. **NAPI Boundary Transfer**: Node.js receives the zero-copy flat metadata buffer and the packed string, and reconstructs the standard PostCSS JS AST classes (`Root`, `Rule`, `Declaration`, `AtRule`, `Comment`) by simple array index offsets and substring extraction.

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

## 📦 Getting Started

### Prerequisites
- Node.js (v18+)
- Rust toolchain (cargo, rustc)

### Installation
Clone the repository and install the development dependencies:
```bash
pnpm install
```

### Building the Native Addon
To build the optimized release target:
```bash
npm run build
```

To build a debug version:
```bash
npm run build:debug
```

### Running Benchmarks
We have included a benchmark suite to compare the speed of the original PostCSS JS parser against `postcss-rs`:
```bash
node benchmark.js
```

## 🧪 Testing and Verification
Unit tests are fully integrated and run against the entire PostCSS test suite to ensure exact parser parity:
```bash
npm test
```
