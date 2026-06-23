# Verification bundle - RFC 0008 spikes

Commands Mirko runs to validate each spike. ship does not run device/deploy steps.

## T00a - rig rmcp builder API + host connect (HOST, Mac)

```bash
# 1. compile (the OQ#1 answer: does the rig 0.36 rmcp builder API compile + version-unify)
cargo build --manifest-path ~/code/flowflow-spikes/t00a-rmcp/Cargo.toml

# 2. ONE rmcp version? (finding 17 - two-version collision gate)
cargo tree -i rmcp --manifest-path ~/code/flowflow-spikes/t00a-rmcp/Cargo.toml

# 3. connect + list tools against a public Streamable-HTTP MCP server
cargo run --manifest-path ~/code/flowflow-spikes/t00a-rmcp/Cargo.toml
#   override endpoint:  cargo run ... -- https://mcp.context7.com/mcp
#   or set OPENAI_API_KEY to also exercise the runtime agent build
```

Pass = build green, `cargo tree -i rmcp` lists exactly one version, run prints "listed N tools" + "DONE".
Fail modes that matter: builder method doesn't compile (API drift) or two rmcp versions (pin needed).

## T00b - aarch64-apple-ios link + on-device connect  (pending T00a green)
(bundle written when T00a passes and Mirko greenlights T00b)

## T02b - token-injection vs github-mcp-server  (pending)
(bundle written when reached)
