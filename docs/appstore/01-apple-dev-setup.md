# 1. Apple Developer Account Setup

## Compte payé vs gratuit

| | Gratuit | Payé (99 EUR/an) |
|---|---|---|
| Provisioning profile | 7 jours | **1 an** |
| Devices | 3 (7j chacun) | **100/plateforme/an** |
| App Store / TestFlight | Non | **Oui** |
| Capabilities (push, iCloud...) | Non | **Oui** |
| Certificats distribution | Non | **Oui** |

## Étapes immédiates (developer.apple.com/account)

### 1. Vérifier l'enrollment
- Page compte → doit afficher "Apple Developer Program" + Team.
- URL : https://developer.apple.com/account

### 2. Enregistrer App ID explicite
- Certificates, Identifiers & Profiles → Identifiers → +
- Type : App IDs → App
- Platform : iOS
- Description : `FlowFlow`
- Bundle ID : **Explicit** → `com.mirkobozzetto.flowflow`
- Capabilities : laisser défaut (In-App Purchase auto-activé, inoffensif)
- Register
- URL : https://developer.apple.com/account/resources/identifiers

### 3. Enregistrer le device
- Méthode simple : Xcode l'ajoute auto au premier build device.
- Manuel : Devices → + → coller UDID iPhone.
- URL : https://developer.apple.com/account/resources/devices

### 4. Créer certificat Apple Development (si pas déjà fait)
- Xcode → Settings → Accounts → sélectionner le compte payé → Manage Certificates → + → Apple Development
- Xcode crée cert + installe dans Keychain automatiquement.
- Vérif : `security find-identity -v -p codesigning | grep "Apple Development"`

### 5. Créer certificat Apple Distribution (BLOQUANT pour App Store)
- Même chemin : Manage Certificates → + → **Apple Distribution**
- Garder UN SEUL cert distribution par team.
- Vérif : `security find-identity -v -p codesigning | grep "Apple Distribution"`

### 6. Profils de provisioning

| Type | Usage | Device list | Durée |
|---|---|---|---|
| Development | `dx serve --ios --device` quotidien | Oui (UDIDs) | 1 an |
| Ad Hoc | Builds test hors TestFlight | Oui (max 100) | 1 an |
| **App Store Connect** | Upload App Store / TestFlight | Non | 1 an |

- Xcode automatic signing crée le profil Development auto.
- Pour App Store : créer manuellement OU laisser Xcode le créer lors de l'Archive → Distribute.
- Manuel : https://developer.apple.com/account/resources/profiles → + → App Store Connect → sélectionner App ID + cert Distribution → Generate → download → double-click.

### 7. Setup Xcode
- Xcode → Settings → Accounts → + → sign in avec le compte payé.
- Pour tout target avec `com.mirkobozzetto.flowflow` :
  - Signing & Capabilities → check "Automatically manage signing"
  - Sélectionner le Team payé (pas "Personal Team")
  - Xcode mint les certs + profils automatiquement.

### 8. Team ID
- Trouver : https://developer.apple.com/account → Membership → Team ID (10 chars alphanumériques).
- dx ne l'utilise pas directement, il passe par le profil.
- Utile pour : `xcodebuild DEVELOPMENT_TEAM=XXXXX`, entitlements.

### 9. Dummy Xcode project
**OBSOLÈTE** avec le compte payé. Le step 7 du CLAUDE.md (workaround provisioning via dummy Swift project) peut être supprimé. Signing auto avec le compte payé suffit.

## Note sur les Apple IDs
Le cert dev existant est signé `Apple Development: mirko@mirko.re (3YL4GA2Y23)`.
Vérifier quel Apple ID détient l'adhésion payante (mirko@mirko.re vs mirko.prodev@gmail.com) avant de générer le cert Distribution.

## Références
- Comparaison memberships : https://developer.apple.com/support/compare-memberships/
- Certificates guide : https://developer.apple.com/help/account/create-certificates/
- Provisioning profiles : https://developer.apple.com/help/account/manage-provisioning-profiles/
- Code signing guide : https://mobileapp.wiki/ios-code-signing-guide
