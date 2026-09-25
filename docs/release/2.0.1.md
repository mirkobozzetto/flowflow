# App Store release 2.0.1

Everything shipped since v1.0 (June 2026), and what to paste where.

`make appstore` sets the numbers itself: it reads `version` from `Cargo.toml`,
bumps the patch, and bumps the build counter in `.appstore-build`. The
`1.0.0` sitting in `Dioxus.toml` is overwritten and can be ignored.

- Version: **2.0.1** — set `Cargo.toml` to `1.9.9` first if you would rather
  announce 2.0.0
- Build: **15**

---

## What's New (App Store field, French)

Le champ « Nouveautés » est limité à 4000 caractères. La version courte
d'abord, la longue en dessous si tu veux tout détailler.

### Version courte

```
Transcription hors ligne

FlowFlow transcrit désormais sans connexion, entièrement sur votre iPhone.
Téléchargez un modèle dans les réglages : votre voix ne quitte plus l'appareil,
et le mode avion suffit.

Vos notes sur tous vos appareils

iPhone et Mac se synchronisent directement entre eux, sur votre réseau local,
sans serveur. Chaque note indique qui l'a écrite.

Un compte FlowFlow

Créez votre compte, liez vos appareils, gérez votre profil et ce que vous
rendez visible, champ par champ.

Partager une note ou un fil

Publiez une note ou une conversation entière derrière un lien. Vous coupez
l'accès quand vous voulez, et ce que les autres ont gardé s'aligne.

Le chat va chercher sur le web

Une question qui dépasse vos notes déclenche une recherche web, fusionnée avec
vos propres notes dans une seule réponse sourcée.

Des rappels depuis le chat

Demandez un rappel en langage courant. Une carte apparaît, vous l'ouvrez ou
vous l'annulez d'un geste.

Un dictionnaire de transcription

Déclarez les noms propres et le jargon que la transcription écorche. La
correction s'applique à tous les moteurs.

Écoute mot à mot

Touchez un mot du transcript, l'audio saute exactement là.

Filtrer d'un geste

Filtrez la liste depuis le champ de recherche : notes vocales, documents,
rappels, fils. Le menu d'action s'ouvre sous votre doigt.

Des agents connectés

Installez des agents vérifiés qui travaillent sur vos notes. Toute écriture
vers un service extérieur vous est soumise avant d'être exécutée.
```

### Version longue, par thème

**Transcription hors ligne.** Un second moteur tourne intégralement sur
l'appareil. Choisissez la taille du modèle, de tiny à large, et transcrivez en
avion. L'audio ne part nulle part.

**Synchronisation entre vos appareils.** iPhone et Mac se trouvent sur le
réseau local et échangent en direct, chiffré, sans serveur au milieu. Les
conflits sont détectés et vous pouvez les trancher. Chaque note porte le nom de
l'appareil qui l'a écrite.

**Compte et profil.** Inscription par passkey, appareils liés au même compte,
profil avec une visibilité réglable champ par champ, et une photo.

**Partage.** Une note ou un fil complet derrière un lien. Vous révoquez quand
vous voulez ; les copies gardées par les autres s'alignent, et une note
supprimée disparaît vraiment, y compris de leur recherche.

**Recherche web dans le chat.** Un interrupteur dans les réglages ajoute une
recherche web en parallèle de vos notes. Les deux classements fusionnent en une
réponse unique, sources visibles. Sans réseau, la réponse reste locale.

**Rappels.** Demandez-les en langage courant depuis le chat. Une carte
récapitule ce qui a été programmé, avec l'annulation à portée. Les rappels
récurrents sont gérés.

**Dictionnaire de transcription.** Vos noms propres, vos termes métier, vos
acronymes. La correction est déterministe et vaut pour tous les moteurs.

**Transcript mot à mot.** Chaque mot porte son horodatage et sa confiance.
Touchez-le, l'audio saute là.

**Filtres et menus.** Le champ de recherche filtre par type de note. Le menu
d'action s'ouvre à l'endroit du doigt, pas en bas de l'écran.

**Agents et connecteurs.** Un annuaire d'agents vérifiés, installables sur
l'appareil, signés et épinglés. Un agent qui veut écrire ailleurs que dans vos
notes vous soumet sa proposition d'abord : rien ne part sans votre accord.

**Sauvegarde et restauration.** Une archive complète, vérifiée, et une
restauration qui reprend là où elle s'arrête si elle est interrompue.

---

## Steps

1. `make appstore` — release build, distribution signing, IPA in `dist/`.
   It validates against Apple's servers when `APPLE_ID` and
   `APP_SPEC_PASSWORD` are set, both of which they are.
2. Open Transporter, drop the IPA, deliver.
3. App Store Connect → the app → **+ Version**, `2.0.1`.
4. Paste the What's New text above.
5. Screenshots: only needed if the UI moved enough that the current ones
   misrepresent it. The account, sync and dictionary screens are new.
6. Submit.

## Before submitting

**Shared spaces are NOT in this release.** They live on `feat/spaces-app`,
unmerged and never run between two real accounts. Shipping them to the public
untested would put a feature nobody has seen work in front of users.

**Signing up still happens on the website.** There is no in-app account
creation, so a new user has to reach `account.flowflow.be` on their own before
anything account-bound works.

**EU distribution is still blocked** on DSA trader verification. v1.0 is live
outside the EU only, and that does not change here.
