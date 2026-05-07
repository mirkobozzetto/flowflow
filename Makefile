.PHONY: build format check dev ddev desktop

build:
	cargo build --features mobile

format:
	cargo fmt

check:
	cargo fmt --check && cargo clippy --features mobile

dev:
	set -a && . ./.env && dx serve --ios

ddev:
	set -a && . ./.env && dx serve --ios --device

desktop:
	set -a && . ./.env && dx serve --desktop

logs:
	idevicesyslog | grep -i "audio\|flowflow\|FlowFlow"
