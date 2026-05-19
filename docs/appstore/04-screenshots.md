# 4. Screenshots et Assets Marketing

## Tailles requises

Depuis septembre 2024, seul le **6.9"** est REQUIS pour iPhone. Apple auto-scale le reste.
FlowFlow = iPhone only → un seul set suffit par langue.

| Device | Résolution | Requis |
|---|---|---|
| **iPhone 6.9"** (16/17 Pro Max) | **1320 x 2868 px** | **OUI** |
| iPhone 6.7" | 1290 x 2796 | Non (auto-scalé) |
| iPhone 6.5" | 1284 x 2778 | Non (auto-scalé) |
| iPhone 6.1" | 1179 x 2556 | Non (auto-scalé) |
| iPhone 5.5" | 1242 x 2208 | Non (auto-scalé) |
| iPad | N/A | Non (iPhone only) |

Format : PNG ou JPG, sRGB ou P3, **JAMAIS CMYK** (rejet).
Taille max : 10 MB par image. Min 1, max 10 screenshots.

## Plan 7 screenshots (story arc)

60% des conversions viennent des screenshots. Users scrollent rarement après le slot 3.
Règles : 1 message/slot, caption ≤7 mots benefit-driven, police ≥48px.

| Slot | Contenu | Caption EN | Caption FR |
|---|---|---|---|
| 1 | Recording bar + waveform | Capture every thought by voice | Capturez chaque idée à la voix |
| 2 | Note transcrite | AI transcribes instantly | Transcription IA instantanée |
| 3 | Chat RAG + sources | Chat with your notes, AI-powered | Discutez avec vos notes par IA |
| 4 | NotesList + tags chips | Auto-tagged and organized | Tags et titres générés auto |
| 5 | Folders / sidebar | Organize in smart folders | Organisez en dossiers intelligents |
| 6 | Attachment import PDF/DOCX | Import PDF, DOCX, search inside | Importez PDF, DOCX, cherchez dedans |
| 7 | Settings provider picker | Your keys, your privacy | Vos clés, votre vie privée |

Si réduction à 5 : garder 1, 2, 3, 4, 6.

### Style
- UI light FlowFlow → fond screenshot **sombre** ou **orange #E85D0A** pour contraste.
- Contenu réel obligatoire (pas de mockup inventé).
- Style "clean interface + text overlay" (comme Notion/Things 3).
- Contenu ≥70% du visuel.

### Capture
- Simulateur iPhone 16/17 Pro Max → résolution native 1320x2868.
- `xcrun simctl io booted screenshot screenshot.png`

## Preview Video

| Spec | Valeur |
|---|---|
| Durée | 15-30 secondes |
| Format | .mov / .mp4 / .m4v |
| Codec | H.264 + AAC |
| Max | 500 MB, 30 fps |
| Contenu | Footage in-app réel uniquement (pas de mockup/mains/CTA/pricing) |
| Position | Autoplay muted, précède les screenshots |
| Max | 3 vidéos/langue |

**Recommandation V1 : skip.** Coût capture+montage élevé. Gain conversion arrive après screenshots solides. À reconsidérer V2.

## App Icon

Spec : 1024x1024 PNG, **ZÉRO alpha** (tous pixels opaques), coins CARRÉS (Apple applique squircle auto), sRGB.

L'icône gear+eyes orange est déjà faite. **Vérification critique** :
```bash
sips -g hasAlpha generated-images/flowflow-icon-transparent.png
```
Si `hasAlpha: yes` → flatten obligatoire. Cause #1 de rejet metadata.

## Promotional Text (170 chars, modifiable sans review)

EN (108 chars) :
> Record voice notes, get instant AI transcription, and chat with your notes. Smart tags, folders, and document import — all on device.

FR (135 chars) :
> Enregistrez vos notes vocales, transcription IA instantanée, et discutez avec vos notes. Tags intelligents, dossiers, import de documents.

## Outils (gratuits, sans compétences design)

| Outil | URL | Prix | Note |
|---|---|---|---|
| **Screenshot Otter** | https://screenshototter.com | Gratuit | AI captions (Claude), localisation 40+ langues, upload direct ASC. **RECOMMANDÉ** |
| **ScreenMaker** | https://screenmaker.app | 100% gratuit | No watermark, no limit |
| Nakxi | https://nakxi.com | Freemium | Templates modernes |
| ScreenKit | https://screenkit.io | Freemium | Frames à jour |

### Workflow
1. Capturer 7 écrans sur simulateur iPhone 16/17 Pro Max (1320x2868 natif)
2. Drop dans Screenshot Otter ou ScreenMaker
3. Ajouter template + captions FR + EN du plan ci-dessus
4. Export ZIP par langue
5. Upload dans App Store Connect → Media Manager → par langue

Temps estimé : ~1-2h hors capture.

## Références
- Apple screenshots guide : https://developer.apple.com/help/app-store-connect/manage-app-information/upload-app-previews-and-screenshots
- Apple icon spec : https://developer.apple.com/design/human-interface-guidelines/app-icons
