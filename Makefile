.PHONY: build fmt check serve serve-device

build:
	cargo build --features mobile

fmt:
	cargo fmt

check:
	cargo fmt --check && cargo clippy --features mobile

serve:
	dx serve --ios

serve-device:
	dx serve --ios --device
