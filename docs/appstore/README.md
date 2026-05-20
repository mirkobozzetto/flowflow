# FlowFlow — App Store Launch Guide

Documentation complète pour publier FlowFlow sur l'App Store iOS.
Générée le 19 mai 2026, basée sur les guidelines Apple en vigueur.

## Documents

| # | Fichier | Contenu |
|---|---------|---------|
| 1 | [apple-dev-setup.md](01-apple-dev-setup.md) | Compte payé, certificats, profils, App ID |
| 2 | [build-signing.md](02-build-signing.md) | Build release Dioxus, IPA, codesign, upload |
| 3 | [appstore-metadata.md](03-appstore-metadata.md) | Nom, description, keywords, catégorie, age rating |
| 4 | [screenshots.md](04-screenshots.md) | Tailles, plan 7 slots, outils, promo text |
| 5 | [testflight.md](05-testflight.md) | Beta testing interne/externe, upload, feedback |
| 6 | [privacy-gdpr.md](06-privacy-gdpr.md) | GDPR, nutrition labels, consentement IA, manifest |
| 7 | [privacy-policy.md](07-privacy-policy-draft.md) | Privacy policy prête à publier (EN + FR) |
| 8 | [i18n.md](08-i18n.md) | Stratégie i18n Dioxus (dioxus-i18n + Fluent) |
| 9 | [checklist.md](09-checklist.md) | Checklist pré-soumission complète |
| 10 | [code-tasks.md](10-code-tasks.md) | Tâches de code à implémenter avant soumission |

## Bloquants critiques (résumé)

1. **Écran consentement IA** — Apple 5.1.2(i) nov 2025. Code à écrire.
2. **Privacy policy live** — Draft prêt, publier sur GitHub Pages.
3. **Certificat Apple Distribution** — Créer dans Xcode.
4. **PrivacyInfo.xcprivacy** — Créer + injecter dans le bundle.
5. **Info.plist fixes** — DTPlatformName, encryption, mic string, UIDeviceFamily.
6. **Clés API reviewer** — Fournir dans App Review Notes.

## Références rapides

- Apple Developer Portal : https://developer.apple.com/account
- App Store Connect : https://appstoreconnect.apple.com
- Dioxus issue #3817 : https://github.com/DioxusLabs/dioxus/issues/3817
- Soniox Console : https://console.soniox.com
- OpenAI API Keys : https://platform.openai.com/api-keys
- Anthropic Console : https://console.anthropic.com
