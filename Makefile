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
	@set -a && . ./.env && set +a; \
	if [ -z "$$APPLE_TEAM_ID" ]; then \
	  echo "ERROR: APPLE_TEAM_ID not set."; \
	  echo "  This target is for App Store distribution (paid Developer Program)."; \
	  echo "  Free users: use 'make all' / 'make ddev' (auto-provisioning Personal Team)."; \
	  echo "  Paid users: add APPLE_TEAM_ID=R477R8NK27 to .env (get it from"; \
	  echo "  https://developer.apple.com/account → Membership → Team ID)."; \
	  exit 1; \
	fi
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
	plutil -replace UIDeviceFamily -json '[1]' \
	  target/dx/flowflow/release/ios/Flowflow.app/Info.plist
	plutil -replace CFBundleSupportedPlatforms -json '["iPhoneOS"]' \
	  target/dx/flowflow/release/ios/Flowflow.app/Info.plist
	@echo ">> Injecting icon (release: re-sign deferred)..."
	APP_PATH=target/dx/flowflow/release/ios/Flowflow.app bash scripts/inject-icon.sh || true
	@echo ">> Replacing main app provisioning profile with App Store distribution..."
	@MAIN_BUNDLE_ID="com.mirkobozzetto.flowflow"; \
	MAIN_PROFILE=""; \
	for f in ~/Library/Developer/Xcode/UserData/Provisioning\ Profiles/*.mobileprovision; do \
	  PLIST=$$(security cms -D -i "$$f" 2>/dev/null || true); \
	  if echo "$$PLIST" | grep -q "$$MAIN_BUNDLE_ID" \
	     && ! echo "$$PLIST" | grep -q "recording-widget" \
	     && ! echo "$$PLIST" | grep -q "ProvisionedDevices"; then \
	    MAIN_PROFILE="$$f"; \
	    break; \
	  fi; \
	done; \
	if [ -z "$$MAIN_PROFILE" ]; then \
	  echo "ERROR: no App Store provisioning profile for $$MAIN_BUNDLE_ID."; \
	  echo "  Create one at https://developer.apple.com/account/resources/profiles/add"; \
	  echo "  Type: App Store. App ID: $$MAIN_BUNDLE_ID. Cert: Apple Distribution."; \
	  echo "  Download and double-click to install in ~/Library/Developer/Xcode/UserData/Provisioning Profiles/"; \
	  exit 1; \
	fi; \
	echo "Using main profile: $$MAIN_PROFILE"; \
	cp "$$MAIN_PROFILE" target/dx/flowflow/release/ios/Flowflow.app/embedded.mobileprovision
	@echo ">> Signing widget for release..."
	bash scripts/sign-widget.sh release
	@echo ">> Injecting PrivacyInfo.xcprivacy..."
	cp ios/PrivacyInfo.xcprivacy target/dx/flowflow/release/ios/Flowflow.app/
	@echo ">> Materializing entitlements with APPLE_TEAM_ID..."
	@set -a && . ./.env && set +a; \
	mkdir -p /tmp/flowflow-build; \
	sed "s/TEAMID/$$APPLE_TEAM_ID/g" ios/entitlements.plist \
	  > /tmp/flowflow-build/entitlements.plist
	@echo ">> Signing main app for distribution..."
	codesign --force --sign "Apple Distribution" \
	  --entitlements /tmp/flowflow-build/entitlements.plist \
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
