.PHONY: build format check dev ddev desktop clean clean-all

build:
	cargo build --features mobile

format:
	cargo fmt

check:
	cargo fmt --check && cargo clippy --features mobile

dev:
	set -a && . ./.env && IPHONEOS_DEPLOYMENT_TARGET=16.0 dx serve --ios

ddev:
	set -a && . ./.env && IPHONEOS_DEPLOYMENT_TARGET=16.0 dx serve --ios --device

desktop:
	set -a && . ./.env && dx serve --desktop

# Device logs: Console.app → select iPhone → filter "FlowFlow"
logs:
	open -a Console

clean:
	rm -rf target/dx target/ios-dev target/desktop-dev target/flycheck0 target/tmp

clean-all:
	cargo clean
