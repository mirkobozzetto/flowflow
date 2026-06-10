.PHONY: build format check dev ddev deploy desktop desktop-build desktop-app desktop-toml restore-ios-toml icon all appstore clean check-profiles renew ensure-profiles

# Strip the iOS-only widget extension from Dioxus.toml: dx 0.7 compiles every
# declared [[ios.widget_extensions]] even for desktop, and the Live Activity
# Swift code does not build outside iOS (issue #20). The original file is
# parked in .Dioxus.toml.ios and restored by trap when dx exits; a leftover
# backup from a crash is restored before the next run.
WIDGETLESS_TOML = awk 'BEGIN{skip=0} /^\[\[ios\.widget_extensions\]\]/{skip=1; next} /^\[/{skip=0} !skip'

APPSTORE_BUILD := $(shell expr $$(cat .appstore-build 2>/dev/null || echo 0) + 1)

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
	bash scripts/inject-url-scheme.sh || true
	bash scripts/inject-icon.sh || true

deploy:
	set -a && . ./.env && IPHONEOS_DEPLOYMENT_TARGET=16.0 dx build --platform ios --device true && bash scripts/inject-url-scheme.sh && bash scripts/inject-icon.sh

# Defensive restore: if a desktop build was killed (SIGKILL/power loss) it can
# leave Dioxus.toml stripped of the iOS widget and the original parked in
# .Dioxus.toml.ios. Restore it before any iOS build so a device build is never
# silently produced without the widget. (Do not run an iOS build while
# `make desktop` is live - it owns that backup for the session.)
restore-ios-toml:
	@[ ! -f .Dioxus.toml.ios ] || { mv .Dioxus.toml.ios Dioxus.toml; echo ">> restored Dioxus.toml from orphaned desktop backup"; }

all: restore-ios-toml ensure-profiles
	set -a && . ./.env && IPHONEOS_DEPLOYMENT_TARGET=16.0 dx build --platform ios --device true
	bash scripts/sign-widget.sh debug
	bash scripts/inject-url-scheme.sh || true
	bash scripts/inject-icon.sh || true

check-profiles:
	@bash scripts/check-profiles.sh

renew:
	@bash scripts/renew-profiles.sh

ensure-profiles:
	@bash scripts/check-profiles.sh > /dev/null 2>&1 || bash scripts/renew-profiles.sh

icon:
	bash scripts/inject-icon.sh

desktop-toml:
	@[ ! -f .Dioxus.toml.ios ] || mv .Dioxus.toml.ios Dioxus.toml
	cp Dioxus.toml .Dioxus.toml.ios
	$(WIDGETLESS_TOML) .Dioxus.toml.ios > Dioxus.toml

desktop: desktop-toml
	set -a && . ./.env && set +a; \
	trap 'mv .Dioxus.toml.ios Dioxus.toml' EXIT INT TERM; \
	dx serve --desktop

desktop-build: desktop-toml
	set -a && . ./.env && set +a; \
	trap 'mv .Dioxus.toml.ios Dioxus.toml' EXIT INT TERM; \
	dx build --platform desktop
	APP_PATH=target/dx/flowflow/debug/macos/Flowflow.app bash scripts/inject-desktop-icon.sh

# Standalone Mac app: release build installed in /Applications, runs without
# any dev server. Data lives in ~/Library/Application Support/FlowFlow.
desktop-app: desktop-toml
	set -a && . ./.env && set +a; \
	trap 'mv .Dioxus.toml.ios Dioxus.toml' EXIT INT TERM; \
	dx build --platform desktop --release
	APP_PATH=target/dx/flowflow/release/macos/Flowflow.app bash scripts/inject-desktop-icon.sh
	rsync -a --delete target/dx/flowflow/release/macos/Flowflow.app/ /Applications/Flowflow.app/
	@echo ">> Flowflow.app installed in /Applications"

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
	plutil -replace CFBundleVersion -string $(APPSTORE_BUILD) \
	  target/dx/flowflow/release/ios/Flowflow.app/Info.plist
	plutil -replace UIDeviceFamily -json '[1]' \
	  target/dx/flowflow/release/ios/Flowflow.app/Info.plist
	plutil -replace CFBundleSupportedPlatforms -json '["iPhoneOS"]' \
	  target/dx/flowflow/release/ios/Flowflow.app/Info.plist
	@echo ">> Injecting App Store required DT* keys (Dioxus dx omits them)..."
	@SDK_VERSION=$$(xcrun --sdk iphoneos --show-sdk-version); \
	SDK_BUILD=$$(xcrun --sdk iphoneos --show-sdk-build-version); \
	OS_BUILD=$$(sw_vers -buildVersion); \
	XCODE_DT=$$(defaults read /Applications/Xcode.app/Contents/Info DTXcode 2>/dev/null); \
	XCODE_DT_BUILD=$$(/usr/libexec/PlistBuddy -c "Print :ProductBuildVersion" "$$(xcode-select -p)/../version.plist" 2>/dev/null); \
	PLIST=target/dx/flowflow/release/ios/Flowflow.app/Info.plist; \
	plutil -replace CFBundlePackageType -string APPL $$PLIST; \
	plutil -replace DTPlatformVersion -string $$SDK_VERSION $$PLIST; \
	plutil -replace DTPlatformBuild -string $$SDK_BUILD $$PLIST; \
	plutil -replace DTSDKName -string iphoneos$$SDK_VERSION $$PLIST; \
	plutil -replace DTSDKBuild -string $$SDK_BUILD $$PLIST; \
	plutil -replace DTXcode -string $$XCODE_DT $$PLIST; \
	plutil -replace DTXcodeBuild -string $$XCODE_DT_BUILD $$PLIST; \
	plutil -replace DTCompiler -string com.apple.compilers.llvm.clang.1_0 $$PLIST; \
	plutil -replace BuildMachineOSBuild -string $$OS_BUILD $$PLIST
	@echo ">> Patching widget Info.plist (arm64, version sync, DT*)..."
	@SDK_VERSION=$$(xcrun --sdk iphoneos --show-sdk-version); \
	SDK_BUILD=$$(xcrun --sdk iphoneos --show-sdk-build-version); \
	OS_BUILD=$$(sw_vers -buildVersion); \
	XCODE_DT=$$(defaults read /Applications/Xcode.app/Contents/Info DTXcode 2>/dev/null); \
	XCODE_DT_BUILD=$$(/usr/libexec/PlistBuddy -c "Print :ProductBuildVersion" "$$(xcode-select -p)/../version.plist" 2>/dev/null); \
	WPLIST=target/dx/flowflow/release/ios/Flowflow.app/PlugIns/recording_widget.appex/Info.plist; \
	plutil -remove UIRequiredDeviceCapabilities $$WPLIST 2>/dev/null || true; \
	plutil -insert UIRequiredDeviceCapabilities -json '["arm64"]' $$WPLIST; \
	plutil -replace CFBundleShortVersionString -string 1.0.0 $$WPLIST; \
	plutil -replace CFBundleVersion -string $(APPSTORE_BUILD) $$WPLIST 2>/dev/null || plutil -insert CFBundleVersion -string $(APPSTORE_BUILD) $$WPLIST; \
	plutil -replace DTPlatformName -string iphoneos $$WPLIST 2>/dev/null || plutil -insert DTPlatformName -string iphoneos $$WPLIST; \
	plutil -replace DTPlatformVersion -string $$SDK_VERSION $$WPLIST 2>/dev/null || plutil -insert DTPlatformVersion -string $$SDK_VERSION $$WPLIST; \
	plutil -replace DTSDKName -string iphoneos$$SDK_VERSION $$WPLIST 2>/dev/null || plutil -insert DTSDKName -string iphoneos$$SDK_VERSION $$WPLIST; \
	plutil -replace DTXcode -string $$XCODE_DT $$WPLIST 2>/dev/null || plutil -insert DTXcode -string $$XCODE_DT $$WPLIST; \
	plutil -replace DTXcodeBuild -string $$XCODE_DT_BUILD $$WPLIST 2>/dev/null || plutil -insert DTXcodeBuild -string $$XCODE_DT_BUILD $$WPLIST; \
	plutil -replace BuildMachineOSBuild -string $$OS_BUILD $$WPLIST 2>/dev/null || plutil -insert BuildMachineOSBuild -string $$OS_BUILD $$WPLIST; \
	plutil -replace LSRequiresIPhoneOS -bool true $$WPLIST 2>/dev/null || plutil -insert LSRequiresIPhoneOS -bool true $$WPLIST; \
	plutil -replace CFBundleSupportedPlatforms -json '["iPhoneOS"]' $$WPLIST 2>/dev/null || plutil -insert CFBundleSupportedPlatforms -json '["iPhoneOS"]' $$WPLIST
	@echo ">> Registering URL scheme (release: re-sign deferred)..."
	APP_PATH=target/dx/flowflow/release/ios/Flowflow.app bash scripts/inject-url-scheme.sh || true
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
	cd /tmp/flowflow-ipa && zip -qry FlowFlow.ipa Payload -x "*.DS_Store" "__MACOSX*"
	cp /tmp/flowflow-ipa/FlowFlow.ipa .
	@echo ">> Validating IPA with Apple altool..."
	@set -a && . ./.env && set +a; \
	if [ -n "$$APPLE_ID" ] && [ -n "$$APP_SPEC_PASSWORD" ]; then \
	  xcrun altool --validate-app -f FlowFlow.ipa -t ios \
	    -u "$$APPLE_ID" -p "$$APP_SPEC_PASSWORD" && \
	  echo ">> Apple validator: PASSED. IPA ready for Transporter." || \
	  { echo ">> Apple validator: FAILED. Fix errors above before upload."; exit 1; }; \
	else \
	  echo ">> Skipping validation (APPLE_ID or APP_SPEC_PASSWORD missing in .env)."; \
	  echo "   Add both to .env to enable server-side validation."; \
	fi
	@echo $(APPSTORE_BUILD) > .appstore-build
	@echo ">> FlowFlow.ipa ready (build $(APPSTORE_BUILD)). Upload via Transporter.app."

clean:
	rm -rf target/dx target/ios-dev target/desktop-dev target/flycheck0 target/tmp
