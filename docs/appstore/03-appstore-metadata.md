# 3. App Store Metadata

## Créer l'app dans App Store Connect

URL : https://appstoreconnect.apple.com

1. Signer les derniers accords (Business section, tax + banking requis même pour app gratuite).
2. Apps → + → New App :
   - Platform : iOS
   - Name : voir suggestions ci-dessous (≤30 chars, **PERMANENT**, réservé à la création)
   - Primary Language : French (ou English, au choix)
   - Bundle ID : `com.mirkobozzetto.flowflow` (sélectionner, **IMMUTABLE**)
   - SKU : `flowflow-001` (interne, pas visible)
   - User Access : Full

## App Name (≤30 chars, INDEXÉ, poids ASO max)

Front-loader le keyword principal après la marque.

| Suggestion | Chars |
|---|---|
| `FlowFlow: Voice Notes AI` | 25 |
| `FlowFlow — AI Voice Notes` | 26 |
| `FlowFlow: Notes Vocales IA` | 28 |

## Subtitle (≤30 chars, INDEXÉ, 2ème poids ASO)

**NE PAS répéter** les mots du Name. Apple ne crédite pas les doublons.
Utiliser `&` au lieu de "and"/"et" pour gagner des chars.

| Suggestion (EN) | Suggestion (FR) |
|---|---|
| `Transcribe, organize & chat` | `Transcrivez, organisez & discutez` |
| `Record, search & summarize` | `Enregistrez, cherchez & résumez` |

## Keywords (100 chars, CACHÉ, INDEXÉ)

Règles :
- Séparés par virgules, **PAS d'espaces après les virgules**
- Singulier uniquement (Apple match le pluriel auto)
- NE PAS répéter les mots du Name/Subtitle
- NE PAS mettre le nom de l'app, la catégorie, "app", des marques concurrentes

Proposition (ajuster selon volume, retirer ce qui est dans Name/Subtitle) :
```
dictation,memo,recorder,speech,text,transcription,journal,assistant,folder,tag,audio,summary,search,idea,organize
```

## Description (4000 chars, PAS indexé, écrire pour la conversion)

- Première phrase = la plus importante (visible avant "more").
- Paragraphe concis + feature list courte.
- Pas de keyword stuffing, pas de prix, pas de "app".

## Promotional Text (170 chars, modifiable SANS review)

EN :
> Record voice notes, get instant AI transcription, and chat with your notes. Smart tags, folders, and document import — all on device.

FR :
> Enregistrez vos notes vocales, transcription IA instantanée, et discutez avec vos notes. Tags intelligents, dossiers, import de documents.

## Category

- **Primary : Productivity** (standard pour note-taking / Notion / Evernote)
- **Secondary : Utilities**

## Age Rating (questionnaire 2026)

Apple a mis à jour le système d'age rating le 31 jan 2026.
Fréquences : `NONE` / `INFREQUENT` / `FREQUENT`.

Pour FlowFlow :
- Tous les content descriptors (violence, sexual, profanity, gambling, drugs) → **NONE**
- `unrestrictedWebAccess` → **false** (pas de browser intégré)
- User-generated content → **oui** (notes = contenu user)
- Chat IA non restreint → **oui**
- Résultat attendu : **pas 4+** (UGC + chat IA = rating plus élevé, probablement 12+ ou 17+)
- **NE PAS mettre dans Kids Category** (incompatible avec envoi IA tiers)

## Pricing

Recommandation V1 : **GRATUIT** sans IAP.

Raisons :
- API keys fournies par l'utilisateur → zéro coût serveur pour le dev.
- Pas de StoreKit à implémenter (simplifie la première soumission).
- Valider le produit d'abord.

Si monétisation future :
- Freemium avec caps (X transcriptions/mois free, illimité en abo ~4-8 EUR/mois).
- OU modèle Obsidian (app gratuite, sync/backup payant).
- Si bundled API keys (clés du dev) → Apple In-App Purchase **obligatoire** (guideline 3.1).

## Support URL (obligatoire)

Même GitHub Pages que la privacy policy. Ex : `https://mirkobozzetto.github.io/flowflow/support.html`
Peut être une simple page avec email de contact.

## App Review Notes

Informations pour le reviewer Apple (champ texte libre lors de la soumission) :

```
FlowFlow requires API keys for AI features (transcription, embeddings, chat).
Test API keys are provided below for review purposes:

- Soniox API Key: [INSERT TEST KEY]
- OpenAI API Key: [INSERT TEST KEY]
- Anthropic API Key: [INSERT TEST KEY] (optional, only if provider set to Anthropic)

To enter keys: launch app → Settings (gear icon) → paste each key.

The app works offline for local notes without API keys. AI features
(transcription, tags, chat) require keys and network access.

The AI consent screen appears on first use. Tap "Enable" to proceed.
Microphone permission is requested when starting a recording.

Native features: on-device SQLite storage, local LanceDB vector search,
native file picker for document import, CoreAudio recording.
No web views, no remote UI.
```

**CRITIQUE** : sans clés de test, le reviewer voit une app cassée → rejet 2.1.

## Export Compliance

FlowFlow utilise HTTPS (reqwest + rustls) = encryption standard exemptée.
`ITSAppUsesNonExemptEncryption = NO` dans Info.plist élimine le questionnaire à chaque upload.

## Review Timeline

- Nouveau : ~24-48h (parfois 72h)
- Updates : ~12-24h
- Metadata only : quelques heures
- Expedited Review : < 24h, uniquement pour urgences réelles (bug critique, sécurité). Max 1-2x/an.
- **Depuis le 28 avril 2026** : les apps doivent être buildées avec iOS 26 SDK (Xcode 26+).

## Références
- App Store Connect workflow : https://developer.apple.com/help/app-store-connect/
- Product page : https://developer.apple.com/app-store/product-page/
- Keywords ASO : https://appcharts.com
- Review guidelines : https://developer.apple.com/app-store/review/guidelines/
- Age rating 2026 : https://developer.apple.com/help/app-store-connect/manage-app-information/set-an-app-age-rating/
