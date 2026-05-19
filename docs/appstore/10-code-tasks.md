# 10. Tâches de code avant soumission

## Track H : Consentement IA (BLOQUANT)

Le plus gros morceau. Apple guideline 5.1.2(i) du 13 nov 2025.

### Fichiers à créer/modifier

| Fichier | Changement |
|---|---|
| `src/ui/consent.rs` | **Nouveau.** Écran de consentement IA (plein écran, 1er usage) |
| `src/ui/mod.rs` | Ajouter module `consent`, afficher avant toute view si pas consenti |
| `src/ui/state.rs` | Ajouter `ai_consent_given: Signal<bool>` à AppState |
| `src/db/settings_repo.rs` | Clé `ai_consent` (timestamp + version) |
| `src/services/transcription.rs` | Guard : vérifier consentement avant envoi Soniox |
| `src/services/llm.rs` | Guard : vérifier consentement avant envoi OpenAI/Anthropic |
| `src/services/embed.rs` | Guard : vérifier consentement avant embedding |
| `src/ui/settings.rs` | Toggle "Fonctionnalités IA" (révocation consentement) |

### Contenu de l'écran

```
FlowFlow utilise des services IA tiers pour :

- Soniox : transcription de vos enregistrements vocaux
- OpenAI : recherche sémantique et chat avec vos notes
- Anthropic : chat alternatif (si sélectionné)

Vos données sont envoyées directement depuis votre appareil
vers ces services via vos propres clés API. Aucune donnée
ne transite par nos serveurs.

Ces services ne utilisent pas vos données pour entraîner
leurs modèles.

[Politique de confidentialité]

        [Activer les fonctionnalités IA]
        [Utiliser sans IA]
```

### Comportement

- Au lancement : si `ai_consent` absent dans settings → afficher l'écran
- "Activer" → stocker `ai_consent = {timestamp, version, providers: [soniox,openai,anthropic]}`
- "Utiliser sans IA" → app fonctionne en mode local (notes, recording, import, pas de transcription/tags/chat)
- Settings → toggle pour révoquer → supprime `ai_consent`
- Chaque service vérifie `ai_consent_given()` avant tout appel réseau

## Fichiers de build à créer

### `ios/entitlements.plist`
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
Remplacer `TEAMID` par le vrai Team ID (10 chars, page Membership).

### `ios/PrivacyInfo.xcprivacy`
Voir contenu complet dans `06-privacy-gdpr.md` section "Privacy Manifest".

## Modifications Info.plist

Trouver où dx 0.7 source le Info.plist (`Dioxus.toml` section `[bundle]` ou généré).
Ajouter/modifier :
- `NSMicrophoneUsageDescription`
- `ITSAppUsesNonExemptEncryption = NO`
- `DTPlatformName = iphoneos`
- `UIDeviceFamily = [1]`
- `CFBundleSupportedPlatforms = [iPhoneOS]`
- `MinimumOSVersion = 16.0`

## Adapter `scripts/inject-icon.sh`

Actuellement hardcode `debug/` + `Apple Development`.
Pour release :
- Paramétrer le chemin (debug vs release)
- Paramétrer l'identité de signature (Development vs Distribution)
- Ajouter injection de `PrivacyInfo.xcprivacy` dans le bundle

## Cible Makefile `appstore`

```makefile
appstore:
	@echo "Building release..."
	set -a && . ./.env && IPHONEOS_DEPLOYMENT_TARGET=16.0 \
	  dx build --platform ios --device --release
	@echo "Fixing Info.plist..."
	# plutil commands to add missing keys
	@echo "Injecting icon..."
	# actool + codesign for release
	@echo "Injecting PrivacyInfo.xcprivacy..."
	# cp into .app bundle
	@echo "Signing distribution..."
	# codesign --force --sign "Apple Distribution: ..."
	@echo "Packaging IPA..."
	# mkdir Payload, cp .app, ditto zip
	@echo "Validating..."
	# xcrun altool --validate-app
	@echo "Done: FlowFlow.ipa"
```

## Clean dead code

| Fichier | Changement |
|---|---|
| `src/models/note.rs` | Retirer `audio_file_path: Option<String>` et `duration_secs: Option<f64>` du struct Note |
| `src/db/note_repo.rs` | Adapter les queries SQL (retirer ces colonnes des SELECT/INSERT/UPDATE) |
| `src/ui/notes/detail.rs` | Retirer `audio_file_path: None, duration_secs: None` des NewTextNote |
| `src/db/schema.rs` | Migration V7 : `ALTER TABLE notes DROP COLUMN audio_file_path; ALTER TABLE notes DROP COLUMN duration_secs;` (SQLite ne supporte DROP COLUMN qu'en 3.35+ / rusqlite bundled OK) |
| `src/db/mod.rs` | Retirer la migration path absolue→relative pour audio_file_path |

**Note** : la migration V5 a déjà migré les données vers `note_audios`. Les colonnes sont des vestiges.

## Lien privacy policy in-app

Dans `src/ui/settings.rs`, ajouter un bouton "Politique de confidentialité" qui ouvre l'URL dans le browser système.

```rust
button {
    onclick: move |_| {
        #[cfg(target_os = "ios")]
        {
            // UIApplication.shared.open(URL)
        }
    },
    "Politique de confidentialité"
}
```

## Soniox EU (nice-to-have)

Dans `src/services/transcription.rs`, rendre le base URL configurable :
- Clé `soniox_region` dans settings_repo (`us` par défaut, `eu` optionnel)
- `api.eu.soniox.com` pour EU data residency
- Toggle dans Settings

## Estimation effort total

| Tâche | Temps estimé |
|---|---|
| Écran consentement IA | ~4-6h |
| Entitlements + manifest + Info.plist | ~2h |
| Adapt inject-icon.sh + Makefile appstore | ~2h |
| Privacy policy lien in-app | ~30min |
| Clean dead code + migration V7 | ~2h |
| i18n (optionnel, pas bloquant V1) | ~1 jour |
| **Total bloquant** | **~12h** |
| **Total avec i18n** | **~2 jours** |
