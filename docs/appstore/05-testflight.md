# 5. TestFlight Beta Testing

## Prérequis
- App record créée dans App Store Connect (bundle ID enregistré).
- Build signé avec profil **App Store Connect** (pas Development).
- `ITSAppUsesNonExemptEncryption = NO` dans Info.plist.

## Internal Testing (recommandé pour démarrer)

- Jusqu'à **100 testeurs** (doivent être des users App Store Connect : Admin, Developer, Marketing...).
- **PAS de Beta App Review.** Build dispo immédiatement après traitement Apple.
- Setup : App Store Connect → app → TestFlight → + Internal Testing → nommer le groupe → **Enable automatic distribution** → Create.
- Ajouter testeurs : ouvrir le groupe → Invite Testers → sélectionner → Add.
- Testeurs reçoivent un email → installent via l'app TestFlight iOS.

Pour FlowFlow solo : commencer avec juste ton Apple ID. Catch les crashs au lancement avant de passer à l'external.

**Attention** : builds uploadés en "TestFlight Internal Only" depuis Xcode ne peuvent PAS être promus en external/App Store. Toujours utiliser "TestFlight & App Store" distribution.

## External Testing

- Jusqu'à **10 000 testeurs** (n'importe quel email/Apple ID).
- **Beta App Review** au premier build de chaque version marketing.
- Timeline review : ~24-48h (parfois 3 jours).
- Setup : TestFlight → + External Testing → nommer → Create → ajouter un build → ajouter testeurs par email ou **public link** (limitable 1-10000, filtrable OS/device).
- Subsequent builds de la même version : skip review en général.
- Max 6 soumissions/24h. Un seul build par version en review à la fois.

## Upload du build

### Option 1 : Transporter.app (RECOMMANDÉ)
- Gratuit sur le Mac App Store : https://apps.apple.com/app/transporter/id1450874784
- Drag & drop l'IPA → sign in → Deliver.
- Le plus simple pour une app non-Xcode.

### Option 2 : altool CLI
```bash
xcrun altool --validate-app -f FlowFlow.ipa --type ios \
  --apiKey <KEYID> --apiIssuer <ISSUER>
xcrun altool --upload-app -f FlowFlow.ipa --type ios \
  --apiKey <KEYID> --apiIssuer <ISSUER>
```
Clé API : App Store Connect → Users and Access → Integrations → App Store Connect API → +.
Fichier `.p8` dans `~/.appstoreconnect/private_keys/AuthKey_<KEYID>.p8`.

### Option 3 : Xcode Organizer (Path A uniquement)
- Si archive Xcode : Window → Organizer → Distribute App → TestFlight & App Store → Upload.

Le build apparaît dans App Store Connect **après traitement Apple** (quelques minutes à quelques heures). Apple envoie un email quand c'est prêt.

## Export Compliance

Sans `ITSAppUsesNonExemptEncryption = NO`, chaque upload affiche "Missing Compliance" et bloque les tests.
FlowFlow = HTTPS/TLS standard (reqwest + rustls) = encryption exemptée → mettre `NO`.

## Test Information (requis avant external review)

TestFlight → sidebar → Additional → Test Information :
- **Beta App Description** (requis) :
  > FlowFlow records voice notes, transcribes them via AI, auto-generates tags/titles, organizes notes in folders, and lets you chat with your notes using semantic search. Beta focus: recording reliability, transcription accuracy, AI tag/title quality, RAG chat relevance, document import (PDF/DOCX/TXT).
- **Feedback Email** (requis) : mirko.prodev@gmail.com
- **What to Test** (par build) : bullet les changements de ce build.
- **Clés API** : indiquer explicitement que le testeur doit entrer ses propres clés OU fournir des clés test dans la description.

## Crash Reports et Feedback

- Testeurs : screenshot → annotate → send (TestFlight 2.3+). Commentaires après crash.
- Console : App Store Connect → TestFlight → Feedback → Screenshots/Crashes. Download .zip. Rétention 120 jours.
- Xcode : Window → Organizer → Feedback + Crashes (symbolicated si dSYMs uploadés).
- **Rust** : les crash reports seront partiellement symboliqués. Uploader les dSYMs/symboles pour des stacks lisibles.

## Expiration 90 jours

- Chaque build expire **90 jours** après upload. L'app cesse de fonctionner.
- **Workaround** : uploader un nouveau build avant expiration. Build-number bump (même version) pour internal = pas de review.
- Bug connu Apple : certains testeurs voient "Beta Has Expired" sur des builds récents. Fix : mise à jour manuelle dans TestFlight ou réinstaller l'app.

## Rejections Beta Review fréquentes

1. **Crash au lancement** (~40% des rejets) — tester sur device réel avant upload.
2. **NSMicrophoneUsageDescription** générique — string spécifique nommant l'app + usage concret.
3. **Privacy policy absente/inaccessible** — URL live dans ASC + lien in-app.
4. **Reviewer ne peut pas tester** — fournir clés API test dans review notes.
5. **Consentement IA manquant** — gate 5.1.2(i) obligatoire.

## Checklist premier upload

1. [ ] App record créée dans App Store Connect
2. [ ] Build signé avec cert Distribution + profil App Store
3. [ ] Version marketing + build number incrémenté
4. [ ] `ITSAppUsesNonExemptEncryption = NO` dans Info.plist
5. [ ] `NSMicrophoneUsageDescription` avec nom app + usage spécifique
6. [ ] Privacy policy URL live et liée dans ASC
7. [ ] Testé sur device réel (pas juste simulateur)
8. [ ] Beta App Description + Feedback Email remplis
9. [ ] Clés API test prêtes pour le reviewer
10. [ ] dSYMs/symboles uploadés
11. [ ] Upload via Transporter ou altool
12. [ ] Internal group d'abord → vérifier → puis external

## Références
- TestFlight : https://developer.apple.com/testflight/
- Beta review : https://developer.apple.com/help/app-store-connect/manage-builds/upload-builds
- Transporter : https://apps.apple.com/app/transporter/id1450874784
