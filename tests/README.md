# Tests — LanceDB iOS Crash Test

Proof that LanceDB compiles and works on iOS targets.

## Results (2026-05-10)

| Test | Target | Crates | Result |
|------|--------|--------|--------|
| macOS host | `aarch64-apple-darwin` | 386 | PASS |
| iOS device | `aarch64-apple-ios` | 399 | PASS |
| iOS simulator | `aarch64-apple-ios-sim` | 492 | PASS |

All tests ran with `lancedb = { version = "0.27.2", default-features = false }`.
Rust 1.94.1, macOS Darwin 25.4.0.

## Critical config

```toml
# Cargo.toml — default-features = false is MANDATORY
# Without it, fp16kernels feature fails to compile on iOS
lancedb = { version = "0.27.2", default-features = false }

# Must match lancedb's arrow version (57.x)
arrow-array = { version = "57", default-features = false }
arrow-schema = { version = "57", default-features = false }
```

## Commands

```bash
# Run integration test (macOS only — no iOS runtime in CI)
cargo test --test lancedb_ios

# Verify iOS compilation
cargo build --target aarch64-apple-ios --features mobile
cargo build --target aarch64-apple-ios-sim --features mobile
```

## What the test proves

1. LanceDB opens a local database (file-based, no server)
2. Arrow schema with FixedSizeList vectors works
3. RecordBatch creation + table insert works
4. Vector similarity search returns ranked results
5. All of the above compiles for aarch64-apple-ios

## Next step

Track E: wire `VectorStore` into `src/services/vectordb.rs` with:
- OpenAI embedding calls (text-embedding-3-small, 1536 dims)
- Chunk notes/documents (512 tokens)
- Store in LanceDB at `~/Documents/flowflow/vectors/`
- Search with folder/tag metadata filtering
