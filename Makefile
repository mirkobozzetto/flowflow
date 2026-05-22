.PHONY: build format check dev ddev deploy desktop icon all appstore clean check-profiles renew ensure-profiles

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

ddev-build:
	set -a && . ./.env && IPHONEOS_DEPLOYMENT_TARGET=16.0 dx build --platform ios --device true
	bash scripts/sign-widget.sh debug
	bash scripts/inject-icon.sh || true

deploy:
	set -a && . ./.env && IPHONEOS_DEPLOYMENT_TARGET=16.0 dx build --platform ios --device true && bash scripts/inject-icon.sh

all: ensure-profiles
	set -a && . ./.env && IPHONEOS_DEPLOYMENT_TARGET=16.0 dx build --platform ios --device true
	bash scripts/sign-widget.sh debug
	bash scripts/inject-icon.sh || true

check-profiles:
	@bash scripts/check-profiles.sh

renew:
	@bash scripts/renew-profiles.sh

ensure-profiles:
	@bash scripts/check-profiles.sh > /dev/null 2>&1 || bash scripts/renew-profiles.sh

icon:
	bash scripts/inject-icon.sh

desktop:
	set -a && . ./.env && dx serve --desktop

# Device logs: Console.app → select iPhone → filter "FlowFlow"
logs:
	open -a Console

appstore:
	@echo ">> Building release..."
	set -a && . ./.env && IPHONEOS_DEPLOYMENT_TARGET=16.0 \
	  dx build --platform ios --device --release
	@echo ">> Patching Info.plist..."
	plutil -replace ITSAppUsesNonExemptEncryption -bool false \
	  target/dx/flowflow/release/ios/Flowflow.app/Info.plist
	plutil -replace DTPlatformName -string iphoneos \
	  target/dx/flowflow/release/ios/Flowflow.app/Info.plist
	plutil -replace LSRequiresIPhoneOS -bool true \
	  target/dx/flowflow/release/ios/Flowflow.app/Info.plist
	plutil -replace MinimumOSVersion -string 16.0 \
	  target/dx/flowflow/release/ios/Flowflow.app/Info.plist
	plutil -replace CFBundleShortVersionString -string 1.0.0 \
	  target/dx/flowflow/release/ios/Flowflow.app/Info.plist
	plutil -replace CFBundleVersion -string 1 \
	  target/dx/flowflow/release/ios/Flowflow.app/Info.plist
	@echo ">> Injecting icon..."
	APP_PATH=target/dx/flowflow/release/ios/Flowflow.app bash scripts/inject-icon.sh || true
	@echo ">> Injecting PrivacyInfo.xcprivacy..."
	cp ios/PrivacyInfo.xcprivacy target/dx/flowflow/release/ios/Flowflow.app/
	@echo ">> Signing for distribution..."
	codesign --force --sign "Apple Distribution" \
	  --entitlements ios/entitlements.plist \
	  target/dx/flowflow/release/ios/Flowflow.app
	@echo ">> Packaging IPA..."
	rm -rf /tmp/flowflow-ipa
	mkdir -p /tmp/flowflow-ipa/Payload
	cp -r target/dx/flowflow/release/ios/Flowflow.app /tmp/flowflow-ipa/Payload/
	cd /tmp/flowflow-ipa && ditto -c -k --sequesterRsrc Payload FlowFlow.ipa
	cp /tmp/flowflow-ipa/FlowFlow.ipa .
	@echo ">> FlowFlow.ipa ready. Upload via Transporter.app."

clean:
	rm -rf target/dx target/ios-dev target/desktop-dev target/flycheck0 target/tmp
