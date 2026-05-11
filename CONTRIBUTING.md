# Contributing to FlowFlow

FlowFlow is an open source project licensed under the [EUPL 1.2](LICENSE).
Contributions are welcome.

## Before you start

- Open an issue to discuss the feature or bug before writing code
- One PR per feature or fix — keep it focused

## Setup

```bash
rustup target add aarch64-apple-ios aarch64-apple-ios-sim
cargo install dioxus-cli
cp .env.example .env
make check
cargo test
```

## Code style

- `cargo fmt` before every commit
- `cargo clippy` must pass with no warnings
- No comments in code — use clear naming instead
- Follow existing patterns in the codebase
- Run `make check` before submitting

## Pull requests

1. Fork the repo and create your branch from `development`
2. Write tests for new functionality (`tests/` directory)
3. `cargo test` must pass (101+ tests, 0 failures)
4. `cargo fmt --check` must be clean
5. Open a PR against `development`, not `main`

## Architecture

- 100% Rust, no JavaScript
- Dioxus 0.7 for iOS UI
- Clean Architecture: models / db / services / platform / ui
- One component per file, one responsibility per module

## License

By contributing, you agree that your contributions will be licensed under the [EUPL 1.2](LICENSE).
