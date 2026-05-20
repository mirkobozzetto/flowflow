# 6. Privacy, GDPR et App Store Compliance

## Architecture privacy de FlowFlow

| Aspect | Statut |
|---|---|
| Serveur backend | **Aucun** |
| Compte utilisateur | **Aucun** |
| Analytics / tracking | **Aucun** |
| Stockage données | **100% local** (SQLite + LanceDB sur device) |
| API keys | **Fournies par l'utilisateur** |
| Données envoyées | Audio → Soniox, texte → OpenAI/Anthropic |

C'est la **meilleure posture privacy possible** pour une app IA. GDPR quasi trivial.

## CRITIQUE : Apple Guideline 5.1.2(i) du 13 nov 2025

URL : https://developer.apple.com/news/?id=ey6d8onl

Apple a ajouté textuellement :
> "You must clearly disclose where personal data will be shared with third parties, including with third-party AI, and obtain explicit permission before doing so."

FlowFlow envoie voix + texte à Soniox/OpenAI/Anthropic = exactement la cible.
**Une privacy policy seule NE SUFFIT PAS.** Il faut un **gate de consentement IA in-app**.

### Implémentation requise (changement de code)

1. **Écran de consentement** au premier usage :
   - Nommer explicitement Soniox, OpenAI, Anthropic
   - Décrire ce qui est envoyé (audio pour transcription, texte pour embeddings/chat)
   - Mentionner "ne servent pas à l'entraînement"
   - Lien vers privacy policy
   - **Default OFF** : l'user tape "Activer"
2. **Gating** : bloquer toute transmission tant que pas consenti
3. **Stockage** : `ai_consent` dans settings_repo SQLite (timestamp, providers, version app)
4. **Révocation** : toggle dans Settings pour désactiver les features IA
5. **UI** : nommer le vendor, pas juste "IA"

## GDPR (Mirko = Bruxelles = EU)

### Rôles
- **Mirko/FlowFlow** = data controller (mais exposition minimale : aucun serveur, aucun accès aux données)
- **Soniox, OpenAI, Anthropic** = data processors
- Comme les users fournissent **leurs propres clés API**, argument fort que l'utilisateur est controller de son propre traitement → responsabilité de Mirko très réduite

### DPA (Data Processing Agreements)

| Provider | DPA | Training | Rétention | EU |
|---|---|---|---|---|
| **Soniox** | Sur demande (support@soniox.com) | **Non** (jamais) | Async API stocke audio dans le namespace du compte. Supprimer post-transcription ou documenter | **EU dispo** : `api.eu.soniox.com` |
| **OpenAI** | Auto-inclus dans API Business Terms + SCCs | **Non** par défaut | ≤30j abuse monitoring, zero-retention possible | US (SCCs pour transfert EU) |
| **Anthropic** | Auto-inclus dans Commercial Terms + SCCs. **Anthropic Ireland Ltd** pour EEA | **Non** | ≤30j | Ireland pour EEA |

URLs DPA :
- OpenAI : https://openai.com/policies/data-processing-addendum
- Anthropic : https://privacy.claude.com/en/articles/7996862
- Soniox : support@soniox.com (demander DPA)
- Soniox security : https://soniox.com/docs/stt/security-and-privacy
- Soniox data residency : https://soniox.com/docs/stt/data-residency

### Actions GDPR proportionnées

1. **Privacy policy publiée** (Art. 13/14 transparence) → voir 07-privacy-policy-draft.md
2. **Consentement** : prompt micro iOS couvre la capture. Gate IA in-app couvre la transmission. Base légale : consentement Art. 6(1)(a)
3. **Minimisation** (Art. 5(1)(c)) : déjà solide (envoi minimal)
4. **Droit à l'effacement** (Art. 17) : supprimer note = cascade SQLite + LanceDB + fichiers audio. Providers auto-suppriment (30j OpenAI/Anthropic, configurable Soniox)
5. **Soniox EU** : recommander `api.eu.soniox.com` (future option Settings). Non bloquant.
6. **DPIA** : non requis légalement (dev solo, pas de traitement systématique à grande échelle). Note interne d'1 page recommandée, pas chère.

### Droit à l'effacement : vérifier le cascade

Quand on supprime une note, vérifier que TOUT est purgé :
- [x] SQLite : note + note_audios (CASCADE) + note_folders + attachments (CASCADE)
- [x] Fichiers WAV sur disque
- [x] LanceDB : `delete_note_embeddings` + `delete_attachment_embeddings`
- [ ] Soniox : audio stocké dans leur API async. Ajouter un appel de suppression post-transcription OU documenter la rétention dans la policy.

## App Store Privacy Nutrition Labels

À remplir dans App Store Connect → App Privacy.

| Data Type | Collecté | Linked to User | Tracking | Purpose |
|---|---|---|---|---|
| **Audio Data** | Oui | Not Linked | Non | App Functionality |
| **Other User Content** (texte, chat, attachments) | Oui | Not Linked | Non | App Functionality |
| Identifiers | Not Collected | — | — | — |
| Usage Data | Not Collected | — | — | — |
| Diagnostics | Not Collected | — | — | — |

"Not Linked" défendable : aucun compte, aucun identifiant device envoyé, aucune analytics.

## Privacy Manifest (`PrivacyInfo.xcprivacy`)

Requis depuis mai 2024, enforced depuis le 12 fév 2025.
À placer à la **racine du bundle** `.app`. Injection post-build (même mécanisme que `make icon`).

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>NSPrivacyTracking</key>
    <false/>
    <key>NSPrivacyTrackingDomains</key>
    <array/>
    <key>NSPrivacyCollectedDataTypes</key>
    <array>
        <dict>
            <key>NSPrivacyCollectedDataType</key>
            <string>NSPrivacyCollectedDataTypeAudioData</string>
            <key>NSPrivacyCollectedDataTypeLinked</key>
            <false/>
            <key>NSPrivacyCollectedDataTypeTracking</key>
            <false/>
            <key>NSPrivacyCollectedDataTypePurposes</key>
            <array>
                <string>NSPrivacyCollectedDataTypePurposeAppFunctionality</string>
            </array>
        </dict>
        <dict>
            <key>NSPrivacyCollectedDataType</key>
            <string>NSPrivacyCollectedDataTypeOtherUserContent</string>
            <key>NSPrivacyCollectedDataTypeLinked</key>
            <false/>
            <key>NSPrivacyCollectedDataTypeTracking</key>
            <false/>
            <key>NSPrivacyCollectedDataTypePurposes</key>
            <array>
                <string>NSPrivacyCollectedDataTypePurposeAppFunctionality</string>
            </array>
        </dict>
    </array>
    <key>NSPrivacyAccessedAPITypes</key>
    <array/>
</dict>
</plist>
```

**ATTENTION** : auditer les crates Rust pour les **Required Reason APIs** :
- `rusqlite` / `std::fs` qui lisent des timestamps de fichiers → catégorie "File Timestamp" (reason C617.1 ou 0A2A.1)
- Apple auto-rejette depuis le 1er mai 2024 si non déclaré dans `NSPrivacyAccessedAPITypes`
- Vérifier aussi : zéro référence IDFA/AdSupport dans les deps Cargo

## App Tracking Transparency

**PAS nécessaire.** ATT requis uniquement pour tracking cross-app/web ou IDFA.
FlowFlow : pas d'analytics, pas d'IDFA, pas d'ad SDK.
NE PAS implémenter ATT, NE PAS ajouter `NSUserTrackingUsageDescription`.

## COPPA / Children

- **NE PAS** mettre dans Kids Category (incompatible avec envoi IA tiers)
- Chat IA non filtré → rating élevé
- Policy indique : non destiné aux moins de 16 ans

## Hébergement Privacy Policy

**GitHub Pages** (gratuit, URL fiable, versionnée).
- Créer repo `mirkobozzetto.github.io` ou activer Pages sur le repo `flowflow`
- URL : `https://mirkobozzetto.github.io/flowflow/privacy.html`
- Même URL dans App Store Connect ET dans Settings in-app
- Le draft complet est dans `07-privacy-policy-draft.md`

## Références
- Apple app privacy : https://developer.apple.com/support/app-privacy-on-the-app-store/
- Privacy manifest : https://developer.apple.com/documentation/bundleresources/privacy_manifest_files
- Required Reason APIs : https://developer.apple.com/documentation/bundleresources/privacy_manifest_files/describing_use_of_required_reason_api
- Guideline 5.1.2(i) AI : https://developer.apple.com/news/?id=ey6d8onl
- OpenAI DPA : https://openai.com/policies/data-processing-addendum
- OpenAI API data usage : https://openai.com/enterprise-privacy/
- Anthropic DPA : https://privacy.claude.com/en/articles/7996862
- Soniox security : https://soniox.com/docs/stt/security-and-privacy
- Soniox data residency : https://soniox.com/docs/stt/data-residency
- GDPR Art. 6 : https://gdpr-info.eu/art-6-gdpr/
- GDPR Art. 28 (processors) : https://gdpr-info.eu/art-28-gdpr/
- Belgian DPA : https://www.autoriteprotectiondonnees.be/
