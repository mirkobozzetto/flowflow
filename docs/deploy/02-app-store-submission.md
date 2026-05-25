# App Store submission — étapes détaillées

Ce guide reprend FlowFlow après `make appstore` (IPA produite) jusqu'à "Submit for Review". Suit l'ordre exact pour éviter le ping-pong entre App Store Connect et terminal.

Pré-requis : `FlowFlow.ipa` existe à la racine du repo. Vérifie :

```bash
ls -lh FlowFlow.ipa
```

Doit afficher ~60 MB.

## Vue d'ensemble

Six étapes, dans cet ordre :

1. Installer Transporter (10 min, une fois)
2. Uploader `FlowFlow.ipa` (5 min upload + 30 min Apple processing)
3. Publier la privacy policy sur GitHub Pages (15 min)
4. Générer les screenshots iPhone 6.9" (30-60 min)
5. Remplir les métadonnées App Store Connect (30-60 min)
6. Submit for Review (1 min)

Total : ~3-4h de travail concentré, plus 24-72h d'attente Apple.

## 1. Installer Transporter

Transporter est l'app officielle Apple pour uploader des IPA vers App Store Connect. Disponible gratuitement sur le Mac App Store.

### Option A — install via Mac App Store GUI

1. Ouvre l'app `App Store` sur ton Mac.
2. Cherche `Transporter`.
3. Clique `Obtenir` puis `Installer`.

### Option B — install via `mas` CLI

`mas` est un CLI third-party pour le Mac App Store. Pas installé sur ta machine :

```bash
brew install mas
mas install 1450874784
```

### Vérification

```bash
ls /Applications/ | grep -i transporter
```

Doit retourner `Transporter.app`.

## 2. Uploader FlowFlow.ipa

### Lancer Transporter

```bash
open -a Transporter
```

Au premier lancement, sign-in avec ton Apple ID Developer (`mirko@mirko.re`). Mot de passe normal, pas app-specific.

Si Apple exige 2FA (code 6 chiffres iPhone) : valide via Settings → Apple ID → Apple ID Verification Code sur ton iPhone.

### Drop l'IPA

1. Fenêtre Transporter ouverte : drag-drop `FlowFlow.ipa` depuis le Finder dans la fenêtre.
2. Transporter valide l'IPA (signature, entitlements, Info.plist) en 30s à 2min. Si erreur → voir Troubleshooting plus bas.
3. Si OK : bouton `Deliver` apparaît → click.
4. Upload prend 1-5 min selon ta connexion.
5. À la fin : statut `Delivered`.

### Apple traite le build

Après `Delivered` : Apple processe le build côté serveur (~15-30 min). Pendant ce temps :

- Email Apple : "Your build is processing".
- Une fois prêt : email "Your build is ready for testing on TestFlight".
- Build apparaît dans App Store Connect → ton app → tab `TestFlight`.

Tu peux continuer les étapes 3-5 pendant le processing.

## 3. Publier la privacy policy

Apple Store Connect exige une **URL publique** vers ta privacy policy. Tu as déjà un draft FR + EN dans `docs/appstore/07-privacy-policy-draft.md`.

Solution simple : GitHub Pages sur un repo dédié.

### Créer le repo flowflow-privacy

```bash
cd /tmp
mkdir flowflow-privacy && cd flowflow-privacy
cp /Users/mirkobozzetto/code/flowflow/docs/appstore/07-privacy-policy-draft.md index.md
git init
git add index.md
git commit -m "Initial privacy policy"
```

Crée le repo distant via GitHub CLI :

```bash
gh repo create mirkobozzetto/flowflow-privacy --public --source=. --push
```

### Activer GitHub Pages

```bash
gh api -X POST /repos/mirkobozzetto/flowflow-privacy/pages \
  -f source[branch]=main -f source[path]=/
```

Ou via web : https://github.com/mirkobozzetto/flowflow-privacy/settings/pages → Source: Deploy from a branch → Branch: `main` / root → Save.

Attendre 1-2 min, ton URL devient :

```
https://mirkobozzetto.github.io/flowflow-privacy/
```

Vérifie dans le browser que la page s'affiche. **Note cette URL**, tu en as besoin à l'étape 5.

## 4. Générer les screenshots

Apple exige 3 à 10 screenshots iPhone 6.9" (1320×2868 px) pour la submission. FR + EN si tu vises les deux langues.

### Préparer le simulateur

```bash
xcrun simctl boot "iPhone 17 Pro Max"
open -a Simulator
```

Vérifie la résolution du device :

```bash
xcrun simctl list devices | grep "iPhone 17 Pro Max"
```

L'iPhone 17 Pro Max boote en 1320×2868. Si tu n'as pas ce simulateur :

```bash
xcrun simctl list runtimes
xcrun simctl create "iPhone 17 Pro Max" "iPhone 17 Pro Max" iOS18.5
```

### Lancer FlowFlow dans le simulateur

```bash
cd /Users/mirkobozzetto/code/flowflow
make dev
```

App se lance. Au premier launch : ConsentScreen → clique `C'est parti !`.

### Préparer du contenu de démo

Pour des screenshots crédibles, seed l'app avec :

- 5-10 notes vocales transcrites (sujets variés : meeting, idée, rappel, todo, citation)
- 1-2 folders (`Travail`, `Personnel`)
- 1 conversation chat avec sources
- 1 attachment PDF importé

5 min de setup manuel. Ne mets PAS de données sensibles (tout ce qui apparaît dans les screenshots sera public).

### Capturer les 7 slots

Stratégie : 1 screenshot = 1 fonctionnalité clé. Ordre narratif :

1. **Recording bar + waveform** — vue d'enregistrement vocal en cours
2. **Note transcrite** — NoteDetail avec titre + transcription + tags
3. **Chat RAG + sources** — ChatView avec question + réponse + sources citées
4. **NotesList** — liste de notes avec tag chips colorés
5. **Folders sidebar** — sidebar ouvert avec arborescence folders
6. **Attachment PDF** — NoteDetail avec carte attachment + modal viewer
7. **Settings** — provider picker OpenAI/Anthropic + API keys

Pour chaque slot, dans le simulateur, navigue à la vue voulue, puis :

```bash
xcrun simctl io booted screenshot ~/Desktop/slot-1.png
```

Répète pour slot-2.png à slot-7.png.

### Vérifier la résolution

```bash
sips -g pixelWidth -g pixelHeight ~/Desktop/slot-1.png
```

Doit afficher `1320 × 2868`. Sinon le simulateur est mal configuré.

### Captions FR + EN (optionnel mais recommandé)

App Store accepte les screenshots bruts ou avec captions (overlay texte explicatif).

Tool simple : **Screenshot Otter** (Mac App Store, payant ~10€) ou **ScreenMaker** (gratuit). Permet d'ajouter des captions par-dessus.

Sans tool : tu uploades les screenshots bruts. C'est OK pour V1.

### Output attendu

```
~/Desktop/
  slot-1.png  (1320×2868)
  slot-2.png  (1320×2868)
  ...
  slot-7.png  (1320×2868)
```

Tu peux faire la même chose pour EN plus tard (relance l'app en mode EN une fois i18n fait).

## 5. Remplir les métadonnées App Store Connect

Retour sur https://appstoreconnect.apple.com/apps → clique sur ton app `FlowFlow` → sidebar `App iOS Version 1.0`.

### 5a. Captures d'écran iPhone

Sur la page Version 1.0 (où tu es actuellement), section `Aperçus et captures d'écran` :

1. Onglet `iPhone` sélectionné.
2. Drag-drop tes 7 PNG depuis `~/Desktop/` dans la zone `Faites glisser jusqu'à 10 captures d'écran ici`.
3. Apple ré-ordonne par drag. Slot 1 = première impression (recording bar).

Apple n'exige pas les 10. **3 suffisent** pour soumettre, mais 7 = meilleur. Slot 1 et 2 sont les plus vus dans les résultats de recherche App Store.

### 5b. Texte promotionnel + description + keywords

Scroll bas sur la même page :

| Champ | Limite | Valeur suggérée |
|-------|--------|-----------------|
| `Texte promotionnel` | 170 chars | `Notes vocales transcrites par IA, organisées par tags et folders, avec chat RAG sur tes notes. 100% local sur ton iPhone.` |
| `Description` | 4000 chars | voir bloc ci-dessous |
| `Mots-clés` | 100 chars | `voice,notes,AI,chat,RAG,transcription,audio,Soniox,GPT,Claude` |
| `URL d'assistance` | URL | `https://github.com/mirkobozzetto/flowflow` |
| `URL marketing` | URL (optionnel) | vide pour V1 |

#### Description complète (copier-coller)

```
FlowFlow transforme tes pensées en notes structurées en quelques secondes.

ENREGISTRE
Appuie sur l'icône micro. Parle. FlowFlow capture ta voix avec une qualité audio iOS native, puis Soniox transcrit en français en quelques secondes.

ORGANISE
Tags auto-générés par IA. Folders hiérarchiques. Recherche sémantique sur tes notes via embeddings OpenAI locaux. Tout vit sur ton iPhone.

DISCUTE AVEC TES NOTES
Pose une question. FlowFlow trouve les passages pertinents et te répond en citant ses sources. Provider au choix : OpenAI (GPT) ou Anthropic (Claude).

IMPORTE DES DOCUMENTS
Ajoute des PDF, DOCX, MD ou CSV à une note. FlowFlow les indexe et les rend cherchables avec le reste.

LOCAL D'ABORD
Tes données restent sur ton iPhone. SQLite + LanceDB locaux. Les seuls appels externes sont vers les API que tu configures explicitement (transcription, IA).

100% RUST
Construit avec Dioxus, cpal, rig-core, LanceDB. Performance native, mémoire optimisée, zéro JavaScript.

GRATUIT, OPEN SOURCE
Code source disponible. Apporte tes propres clés API (OpenAI, Anthropic, Soniox).
```

### 5c. Sidebar gauche — autres pages

Sidebar gauche → clique `Informations sur l'app` :

- **Sous-titre** (30 chars) : `Notes vocales + AI chat`
- **Catégorie principale** : `Productivité`
- **Catégorie secondaire** : `Utilitaires`
- **Droits sur le contenu** : `Non, mon app ne contient pas, n'affiche pas et n'accède pas à du contenu de tiers`
- **Classification d'âge** : clique `Modifier` → questionnaire (réponds `Aucun` à tout sauf Apps with User-Generated Content → `Modéré` ou `Non` selon ton interprétation, FlowFlow étant local-only et solo) → Done

Sidebar → `Tarification et disponibilité` :

- **Prix** : `Gratuit (CHF 0.00 / EUR 0.00)`
- **Disponibilité** : tous les pays par défaut, ou restreins aux pays UE/Suisse si tu préfères phase de lancement progressif

Sidebar → `Confidentialité de l'app` :

- **URL de la politique de confidentialité** : colle l'URL GitHub Pages de l'étape 3
- **Pratiques en matière de données** : clique `Commencer`
  - **Données audio** : Oui collectées
    - Liées à l'utilisateur : Non (FlowFlow ne lie pas l'audio à un compte)
    - Suivi : Non
    - Utilité : `Fonctionnalités de l'app` (transcription)
  - **Contenu utilisateur (notes)** : Oui collectées
    - Liées à l'utilisateur : Non
    - Suivi : Non
    - Utilité : `Fonctionnalités de l'app`
  - **Identifiants** : Non
  - **Données d'utilisation** : Non
  - **Diagnostics** : Non
  - **Coordonnées** : Non
  - **Données financières** : Non
  - **Santé et fitness** : Non
  - **Achats** : Non
  - **Recherches** : Non

Save chaque section.

### 5d. Vérification de l'app

Sidebar → `Vérification de l'app` :

- **Compte de connexion requis** : `Non` (FlowFlow n'a pas de login)
- **Notes** : texte pour le reviewer Apple :

```
FlowFlow is a voice-note app with AI transcription and RAG chat.

To test:
1. Launch the app — a consent screen appears explaining AI data flows. Tap "C'est parti !" to accept.
2. Tap the floating + button to create a note.
3. Tap the microphone icon to record a voice note. Speak for 5-10 seconds, then stop.
4. The transcription takes ~3 seconds (Soniox API).
5. Open the Settings tab to configure your own API keys, or use the test keys below.
6. From the home screen, tap the chat icon to ask a question about your notes.

Test API keys are pre-configured in the build. No additional sign-in needed.

The app does not collect personal data and does not track users. All notes are stored locally on the device (SQLite + LanceDB).

System requirements: iOS 16.0+. iPhone only.
```

- **Coordonnées** : nom (Mirko Bozzetto), email (`mirko@mirko.re`), téléphone (+32 484 906 499)
- **Compte de démonstration** : laisse vide (pas de login dans FlowFlow)
- **Notes supplémentaires** : laisse vide

### 5e. Build TestFlight

Sidebar → reviens sur la page `App iOS Version 1.0`. Scroll vers `Build`.

Si Apple a fini de processer ton IPA (étape 2), il apparaît ici. Clique `Sélectionner un build` → choisis `1.0 (1)`.

Si rien n'apparaît : attends. Le processing prend jusqu'à 1h dans les cas extrêmes.

### 5f. Conformité à l'export

Section `Conformité aux exigences en matière d'exportation` (sous Build) :

- Question : `Votre app utilise-t-elle uniquement les algorithmes de chiffrement standard d'iOS ?` → **Oui**
  - Justification : FlowFlow utilise HTTPS via `reqwest` pour appeler les API externes (OpenAI, Anthropic, Soniox), mais ne fait pas d'implémentation custom de chiffrement.

### 5g. IDFA (Advertising)

Section `Identifiant publicitaire (IDFA)` :

- `Cette app utilise-t-elle l'identifiant publicitaire (IDFA) ?` → **Non**

## 6. Submit for Review

Une fois toutes les sections ci-dessus remplies (sidebar gauche : icônes ✓ vert partout), clique le bouton `Ajouter pour vérification` en haut à droite.

Apple confirme et passe le statut à `Waiting for Review` (24-72h typique), puis `In Review`, puis `Ready for Distribution` (si OK) ou `Rejected` (avec raisons).

En cas de rejet :

- Lis attentivement le feedback Apple (envoyé par email + visible dans `Vérification de l'app`)
- Corrige le code ou les métadonnées
- Re-soumets : `Add for Review` redevient cliquable

## Troubleshooting Transporter

### `ITMS-90717: Invalid App Store Icon`

L'icône 1024×1024 a un canal alpha. Tu l'as déjà stripé (`make appstore` produit un IPA correct), mais si tu reconstruis depuis zéro : voir `AppIcon.xcassets/AppIcon.appiconset/icon-1024.png` et vérifie `sips -g hasAlpha icon-1024.png` → `no`.

### `ITMS-90209: Invalid Segment Alignment`

Le binaire n'a pas été correctement signé pour distribution. Probable mauvaise cert (Apple Development au lieu de Apple Distribution). Re-run `make appstore` après vérification :

```bash
security find-identity -v -p codesigning | grep "Apple Distribution"
```

### `Missing Push Notification Entitlement`

Tu as activé Push Notifications dans Apple Developer Portal mais l'entitlement n'est pas dans `ios/entitlements.plist`. FlowFlow n'utilise pas Push pour V1 → désactive le capability sur le App ID dans Apple Developer Portal.

### `Invalid Provisioning Profile`

Le `.mobileprovision` embarqué dans l'IPA n'est pas un App Store profile. Vérifie :

```bash
unzip -p FlowFlow.ipa "Payload/Flowflow.app/embedded.mobileprovision" \
  | security cms -D -i - | grep -A1 ProvisionsAllDevices
```

Si tu vois `<true/>` ou la clé `ProvisionedDevices` → c'est un dev profile, pas App Store. Refais l'étape 5 du guide `01-fresh-setup-from-scratch.md`.

## Backup minimal recommandé

Avant de cliquer Submit, copie en lieu sûr :

- `secrets/ios/distribution.key` (1Password ou disque chiffré)
- `secrets/ios/distribution.cer`
- Les 2 `.mobileprovision`
- Ton Team ID : `R477R8NK27`
- Tes bundle IDs : `com.mirkobozzetto.flowflow` + `.recording-widget`

Si tu perds la `.key`, tu ne peux plus jamais signer une mise à jour de FlowFlow sans révoquer le cert et reprendre depuis zéro.

## Timing récap

| Étape | Action | Durée |
|-------|--------|-------|
| 1 | Install Transporter | 10 min |
| 2 | Upload IPA | 5 min + 30 min processing |
| 3 | GitHub Pages privacy policy | 15 min |
| 4 | Screenshots × 7 | 30-60 min |
| 5 | Métadonnées ASC | 30-60 min |
| 6 | Submit | 1 min |
| Apple | Review | 24-72h |
| Total actif | | ~3-4h |
