# 9. Checklist pré-soumission App Store

## A. Apple Developer Portal

- [ ] Enrollment payé actif (https://developer.apple.com/account)
- [ ] App ID explicite `com.mirkobozzetto.flowflow` enregistré
- [ ] Certificat Apple Development créé
- [ ] Certificat **Apple Distribution** créé
- [ ] iPhone enregistré dans Devices
- [ ] Team ID noté (Membership page)

## B. Xcode

- [ ] Compte payé ajouté dans Xcode → Settings → Accounts
- [ ] Automatic signing activé avec Team payé
- [ ] Profil Development auto-généré (1 an)
- [ ] Profil App Store Connect auto-généré (lors de l'Archive)

## C. Code (changements requis)

- [ ] **Écran de consentement IA** (Apple 5.1.2(i) du 13 nov 2025)
  - Nommer Soniox, OpenAI, Anthropic
  - Default OFF, user tape "Activer"
  - Stocké dans settings_repo SQLite (`ai_consent`)
  - Bloquer transmission tant que pas consenti
  - Toggle révocation dans Settings
- [ ] **Créer `ios/entitlements.plist`** (get-task-allow=false)
- [ ] **Créer `PrivacyInfo.xcprivacy`** (Audio + Other User Content)
- [ ] Auditer crates pour Required Reason APIs
- [ ] Vérifier zéro IDFA/AdSupport dans Cargo deps
- [ ] Lien privacy policy dans Settings (bouton ouvrant l'URL)
- [ ] Clean dead code : `Note.audio_file_path` + `Note.duration_secs`

## D. Info.plist / Build

- [ ] `NSMicrophoneUsageDescription` nommant l'app + usage spécifique
- [ ] `ITSAppUsesNonExemptEncryption = NO`
- [ ] `DTPlatformName = iphoneos`
- [ ] `LSRequiresIPhoneOS = true`
- [ ] `MinimumOSVersion = 16.0`
- [ ] `UIDeviceFamily = [1]` (iPhone only, pas [1,2])
- [ ] `CFBundleSupportedPlatforms = [iPhoneOS]` (pas [iPhoneOS, iPadOS])
- [ ] `CFBundleVersion` incrémenté (pas figé à 0.1.0)
- [ ] `CFBundleShortVersionString = 1.0.0`

## E. Build et signing

- [ ] `dx build --platform ios --device --release` compile
- [ ] Binary = arm64 aarch64-apple-ios (vérif : `file flowflow`)
- [ ] Codesign avec cert **Apple Distribution** (pas Development)
- [ ] IPA packagée (Payload/ + ditto)
- [ ] IPA validée (`altool --validate-app`)
- [ ] `inject-icon.sh` adapté pour release + Distribution
- [ ] Cible Makefile `appstore` fonctionnelle

## F. Assets

- [ ] Icône 1024x1024 PNG sans alpha (`sips -g hasAlpha`)
- [ ] Screenshots 1320x2868 (iPhone 6.9") x7 slots
- [ ] Screenshots générés via Screenshot Otter ou ScreenMaker

## G. Privacy

- [ ] Privacy policy publiée (GitHub Pages)
- [ ] URL live et fonctionnelle
- [ ] Privacy policy liée dans App Store Connect
- [ ] Privacy policy accessible in-app (Settings)
- [ ] `PrivacyInfo.xcprivacy` injecté dans le bundle
- [ ] DPA OpenAI accepté (compte API)
- [ ] DPA Anthropic accepté (Commercial Terms)
- [ ] DPA Soniox demandé (support@soniox.com)

## H. App Store Connect

- [ ] App record créée (nom, bundle ID, SKU, langue)
- [ ] Metadata remplie (nom, subtitle, description, keywords)
- [ ] Category : Productivity + Utilities
- [ ] Age rating questionnaire complété (UGC + chat IA)
- [ ] Privacy nutrition labels (Audio Data + Other User Content)
- [ ] Support URL live
- [ ] Marketing URL (optionnel)
- [ ] Promotional text FR + EN
- [ ] Screenshots uploadés
- [ ] Pricing : Free

## I. Soumission

- [ ] Build uploadée (Transporter.app ou altool)
- [ ] Build traitée (email Apple reçu)
- [ ] Build assignée à la version
- [ ] Export Compliance answered (ou via Info.plist)
- [ ] Content Rights : "does not contain third-party content" (ou "yes, I have rights")
- [ ] IDFA : "no"
- [ ] App Review Notes rédigées (clés API test, description du flow consentement)
- [ ] **Clés API test incluses dans les notes**
- [ ] Submit for Review

## J. Post-soumission

- [ ] Monitoring : status Waiting for Review → In Review → Approved/Rejected
- [ ] Si rejet : lire la raison, fixer, resoumettre
- [ ] Si approuvé : vérifier listing live sur l'App Store
- [ ] Mettre à jour `remember.md` + CLAUDE.md

## Ordre recommandé

1. **Portal** : App ID + certs (A)
2. **Code** : consentement IA + entitlements + manifest (C)
3. **Build** : Info.plist fixes + release build (D, E)
4. **Privacy** : publier policy, remplir nutrition labels (G)
5. **Assets** : icône vérif + screenshots (F)
6. **ASC** : metadata + upload + submit (H, I)
