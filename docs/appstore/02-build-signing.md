# 2. Build, Signing et Upload

## État actuel de dx

`dx` 0.7 n'a **pas** de pipeline App Store. Pas d'archive, pas de signing distribution, pas d'upload.
Docs Dioxus : "Dioxus doesn't provide built-in utilities for [signing and notarizing]."

Mais dx a des **flags de codesign non documentés** (trouvés dans le source `packages/cli/src/cli/target.rs`) :
- `--codesign`
- `--apple-entitlements <path>`
- `--apple-team-id "Nom (TEAMID)"`

Issue #3817 (OPEN) : https://github.com/DioxusLabs/dioxus/issues/3817
Le bug #3817 est un **faux blocage** : le binaire est correct arm64, c'est juste le Info.plist qui manque des clés.

## Path A : Xcode Archive Wrapper (RECOMMANDÉ)

Le plus robuste. Signing automatique, symboles inclus, upload intégré.

### Étapes

1. **Prérequis** : App ID + cert Distribution + Xcode auto-signing (voir 01-apple-dev-setup.md)

2. **Build release** :
```bash
set -a && . ./.env && IPHONEOS_DEPLOYMENT_TARGET=16.0 \
  dx build --platform ios --device --release
```
Sortie : `target/dx/flowflow/release/ios/Flowflow.app`

3. **Thin Xcode wrapper** :
   - Réutiliser le projet Xcode existant (celui du provisioning).
   - Target iOS, bundle ID `com.mirkobozzetto.flowflow`.
   - Embed/link le binaire + assets dx.
   - Signing & Capabilities → Automatically manage signing → Team payé.

4. **Archive** : Xcode → Product → Archive

5. **Distribute** : Organizer → Distribute App → "App Store Connect" → Upload
   - Xcode crée auto le profil App Store si besoin.
   - Gère le repackaging iphoneos correct (résout #3817).
   - Upload les symboles (dSYM) pour crash reports.

### Avantages Path A
- Signing automatique (pas de codesign manuel)
- Repackaging correct (DTPlatformName, etc.)
- Symboles uploadés
- Un seul clic pour Distribute

## Path B : CLI manuelle (fallback)

Sans Xcode project. Plus fragile.

### Étapes

1. **Build** :
```bash
set -a && . ./.env && IPHONEOS_DEPLOYMENT_TARGET=16.0 \
  dx build --platform ios --device --release \
  --codesign --apple-team-id "Apple Distribution: <NOM> (<TEAMID>)" \
  --apple-entitlements ios/entitlements.plist
```

2. **Patcher Info.plist** (dans le .app) — clés manquantes :
```xml
<key>DTPlatformName</key>
<string>iphoneos</string>
<key>DTSDKName</key>
<string>iphoneos18.0</string>
<key>MinimumOSVersion</key>
<string>16.0</string>
<key>CFBundleSupportedPlatforms</key>
<array><string>iPhoneOS</string></array>
<key>UIDeviceFamily</key>
<array><integer>1</integer></array>
<key>ITSAppUsesNonExemptEncryption</key>
<false/>
```

3. **Embed provisioning profile** :
```bash
cp flowflow_appstore.mobileprovision Flowflow.app/embedded.mobileprovision
```

4. **Codesign distribution** :
```bash
DIST_ID=$(security find-identity -v -p codesigning | grep "Apple Distribution" | head -1 | awk -F'"' '{print $2}')
codesign --force --sign "$DIST_ID" \
  --entitlements ios/entitlements.plist \
  --timestamp Flowflow.app
codesign --verify --strict --verbose=2 Flowflow.app
```

5. **Packager en IPA** (utiliser `ditto`, pas `zip` — préserve symlinks) :
```bash
mkdir Payload
cp -R Flowflow.app Payload/
ditto -c -k --sequesterRsrc --keepParent Payload FlowFlow.ipa
```

6. **Upload** :
```bash
# Option 1 : Transporter.app (GUI, recommandé pour le premier upload)
# Télécharger depuis le Mac App Store, drag & drop l'IPA.

# Option 2 : altool CLI
xcrun altool --validate-app -f FlowFlow.ipa --type ios \
  --apiKey <KEYID> --apiIssuer <ISSUER>
xcrun altool --upload-app -f FlowFlow.ipa --type ios \
  --apiKey <KEYID> --apiIssuer <ISSUER>

# Clé API : App Store Connect → Users and Access → Integrations → App Store Connect API → +
# Fichier .p8 à placer dans ~/.appstoreconnect/private_keys/AuthKey_<KEYID>.p8
```

## Entitlements (`ios/entitlements.plist`)

À créer dans le projet. Contenu minimal :

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>application-identifier</key>
    <string>TEAMID.com.mirkobozzetto.flowflow</string>
    <key>com.apple.developer.team-identifier</key>
    <string>TEAMID</string>
    <key>get-task-allow</key>
    <false/>
</dict>
</plist>
```

`get-task-allow = false` est **obligatoire** pour distribution. Les profils Development le mettent à `true`, l'App Store rejette `true`.

## Info.plist — clés obligatoires

```xml
<key>NSMicrophoneUsageDescription</key>
<string>FlowFlow uses the microphone to record your voice notes, which are transcribed and organized in the app.</string>
<key>ITSAppUsesNonExemptEncryption</key>
<false/>
<key>DTPlatformName</key>
<string>iphoneos</string>
<key>LSRequiresIPhoneOS</key>
<true/>
<key>MinimumOSVersion</key>
<string>16.0</string>
<key>UIDeviceFamily</key>
<array><integer>1</integer></array>
<key>CFBundleSupportedPlatforms</key>
<array><string>iPhoneOS</string></array>
```

## Versioning

- `CFBundleShortVersionString` = version marketing (ex: `1.0.0`). Visible sur l'App Store.
- `CFBundleVersion` = build number. **DOIT incrémenter à chaque upload.** App Store Connect rejette les doublons.
- Convention : `1.0.0` (marketing) + `1`, `2`, `3`... (build) ou timestamp `20260519.1`.

## Adapt inject-icon.sh

Le script actuel hardcode `debug/` + `Apple Development`. Pour release :
- Chemin : `target/dx/flowflow/release/ios/Flowflow.app`
- Identité : `Apple Distribution: <NOM> (<TEAMID>)`
- Créer une cible Makefile `appstore` séparée.

## Cible Makefile proposée

```makefile
appstore:
	set -a && . ./.env && IPHONEOS_DEPLOYMENT_TARGET=16.0 \
	dx build --platform ios --device --release
	# Fix Info.plist
	# Inject icon
	# Inject PrivacyInfo.xcprivacy
	# Codesign distribution
	# Package IPA
	# Validate
	@echo "IPA prête : FlowFlow.ipa"
```

## Symboles / Crash Reports

Pour que les crash reports Rust soient lisibles :
- Path A : Xcode upload les dSYMs automatiquement.
- Path B : uploader manuellement via `altool --upload-symbols` ou App Store Connect.
- Rust panics apparaissent comme signaux génériques sans symboles.

## CI/CD (optionnel, GitHub Actions)

Runner `macos-15` :
1. Import cert P12 + profil en base64 (secrets)
2. `dx build --platform ios --device --release`
3. Fix Info.plist + inject icon/manifest
4. `ditto` IPA
5. `altool --upload-app` avec clé API ASC

Caveat : vérifier que lancedb/rusqlite-bundled/pdf-extract cross-compilent sur le runner.

## Références
- Dioxus bundling : https://dioxuslabs.com/learn/0.7/tutorial/bundle/
- Dioxus deploy : https://dioxuslabs.com/learn/0.7/tutorial/deploy/
- Issue #3817 : https://github.com/DioxusLabs/dioxus/issues/3817
- altool : https://keith.github.io/xcode-man-pages/altool.7.html
- Apple distribution : https://developer.apple.com/documentation/xcode/distributing-your-app-for-beta-testing-and-releases
- Transporter : https://apps.apple.com/app/transporter/id1450874784
- App Store Connect API : https://developer.apple.com/documentation/appstoreconnectapi
