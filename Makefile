.PHONY: build format check dev ddev desktop

build:
	cargo build --features mobile

format:
	cargo fmt

check:
	cargo fmt --check && cargo clippy --features mobile

dev:
	dx serve --ios

ddev:
	dx serve --ios --device

desktop:
	dx serve --desktop
