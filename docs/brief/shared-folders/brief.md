---
feature: Shared folders (collaborative spaces)
slug: shared-folders
type: brief
status: draft
stepsCompleted: [0, 1, 2]
issue: marketplace-flowflow#88
---

# Shared folders (espaces collaboratifs)

## En bref

Pour une petite équipe qui veut un cerveau commun, pas un dossier
figé envoyé une fois. Aujourd'hui un dossier ne se partage pas : le
partage s'arrête à la note et au thread (#86). Cette fonctionnalité
livre un espace vivant, où chacun dépose ses notes, où les
permissions descendent aux sous-dossiers, et où sortir du groupe
n'efface jamais son propre travail.

## Problem statement

Le partage livré en 2026-08 couvre une note ou un thread : une unité
ponctuelle, sans vie propre. L'unité naturelle de FlowFlow est le
dossier, le thème. Quand deux personnes veulent travailler ensemble,
elles n'ont aujourd'hui que l'envoi note par note, refait à chaque
ajout, sans mise à jour, sans droit d'écriture, sans révocation.
L'application reste mono-utilisateur alors que ses briques d'identité
et de partage sont déjà là.

## Goals

- Un dossier partagé reste vivant : ce qui est ajouté après le partage
  arrive chez les autres membres.
- Un propriétaire décide, dossier par dossier, entre lecture seule et
  collaboratif ; ce choix descend aux sous-dossiers.
- Rejoindre un espace tient en un geste : un lien ou un QR code, sans
  passer par un site tiers.
- Une note qui arrive dans un téléphone se comporte comme toute note
  FlowFlow : indexée, cherchable, utilisable en chat.
- Quitter ou être révoqué laisse à la personne une copie de ce qu'elle
  a créé, et retire ses notes de l'espace si elle le demande.
- Supprimer une note la supprime partout, y compris de l'index
  vectoriel, chez tous les membres.

## Out-of-scope

- Rôles fins (admin, modérateur, invité). V1 : le créateur invite et
  révoque, les membres participent.
- Invitation par un membre non-créateur.
- Édition d'une note dont on n'est pas l'auteur.
- Android, Windows, Linux. iOS d'abord, macOS suit dans la même
  fonctionnalité.
- Chiffrement de bout en bout de l'espace.
- Marketplace, agents, connecteurs à l'échelle de l'espace.
- Fusion ou déplacement d'un dossier existant vers un espace partagé
  (v1 : on crée le dossier dans l'espace).

## User stories

1. En tant que créateur, je veux transformer un dossier en espace
   partagé, pour que mon équipe y travaille avec moi.
2. En tant que créateur, je veux marquer un dossier ou un
   sous-dossier lecture seule ou collaboratif, pour contrôler qui
   écrit où.
3. En tant que membre, je veux rejoindre un espace via un lien ou un
   QR code reçu, pour entrer sans procédure.
4. En tant que personne sans compte, je veux créer mon compte depuis
   l'application, pour pouvoir rejoindre un espace.
5. En tant que membre, je veux voir arriver les notes des autres,
   pour travailler sur un contenu à jour.
6. En tant que membre, je veux créer mes propres sous-dossiers dans
   un dossier collaboratif, lecture seule ou collaboratifs, pour
   organiser ma part.
7. En tant que membre, je veux chercher et discuter avec les notes de
   l'espace comme avec les miennes, pour que le partage serve à
   quelque chose.
8. En tant que créateur, je veux révoquer un membre, pour fermer
   l'accès.
9. En tant que membre sortant ou révoqué, je veux récupérer mes notes
   dans un dossier local, pour ne rien perdre.
10. En tant qu'auteur, je veux qu'une note supprimée disparaisse
    partout, index compris, pour tenir mon obligation d'effacement.

## Acceptance criteria

Espace et permissions (stories 1, 2, 6)

- [ ] Le menu d'une ligne de dossier propose de le partager, à côté de
      renommer, déplacer, supprimer.
- [ ] À la création comme après coup, un dossier porte un mode visible :
      lecture seule ou collaboratif.
- [ ] Un sous-dossier créé sans choix explicite hérite du mode de son
      parent.
- [ ] Dans un parent collaboratif, un membre peut créer un sous-dossier
      lecture seule ; personne d'autre que son auteur n'y écrit.
- [ ] Un parent lecture seule n'accepte aucun sous-dossier collaboratif.
- [ ] Un membre voit, avant d'écrire, s'il a le droit d'écrire ici.

Adhésion (stories 3, 4)

- [ ] Le créateur génère une invitation partageable comme lien et comme
      QR code, et peut l'envoyer par email.
- [ ] Scanner le QR code ou ouvrir le lien depuis un téléphone où
      FlowFlow est installé mène à l'écran « rejoindre cet espace ».
- [ ] Sans compte activé, l'application propose la création de compte,
      puis reprend l'adhésion là où elle s'était arrêtée.
- [ ] Le même lien ouvert sur une machine sans FlowFlow explique quoi
      faire, il ne mène pas à une page morte.
- [ ] Le nom affiché d'un membre vient de son profil public.

Vie de l'espace (stories 5, 7)

- [ ] Une note ajoutée par un membre apparaît chez les autres à
      l'ouverture de l'application ou du dossier, sans action manuelle.
- [ ] Hors ligne, l'espace reste consultable avec ce qui a déjà été
      reçu, et l'état de fraîcheur est visible.
- [ ] La synchronisation ne redemande pas ce qui n'a pas changé.
- [ ] Une note reçue est cherchable et utilisable en chat comme une
      note locale.
- [ ] Chaque note affiche son auteur.
- [ ] Un membre ne peut modifier que ses propres notes.

Sortie et effacement (stories 8, 9, 10)

- [ ] Le créateur peut révoquer un membre ; l'accès est fermé au
      prochain contact serveur.
- [ ] En quittant ou en étant révoqué, la personne se voit proposer de
      conserver ses notes : elles sont recopiées dans un dossier local
      hors de l'espace.
- [ ] La personne peut demander le retrait de ses notes de l'espace ;
      elles disparaissent alors chez tous les autres membres.
- [ ] Une note retirée disparaît aussi de l'index vectoriel de chaque
      appareil : elle ne ressort ni en recherche ni en chat.
- [ ] Supprimer l'espace le fait disparaître chez tous les membres, et
      chacun garde l'option de récupérer ses propres notes.

## Success metrics

Fonctionnalité neuve, pas de baseline d'usage. La métrique primaire est
une métrique de preuve, mesurée sur une session de test avec un
deuxième utilisateur réel, fenêtre d'une semaine :

- Primaire : 1 espace, 2 comptes, 2 appareils, au moins 20 notes
  échangées dans les deux sens, 0 divergence constatée après
  révocation et après retrait de notes (0 note fantôme en recherche ou
  en chat côté lecteur).
- Adhésion : parcours lien ou QR jusqu'à la première note visible en
  moins de 2 minutes, création de compte comprise.
- Garde-fou fraîcheur : une note ajoutée est visible chez l'autre
  membre en moins de 60 secondes après ouverture de l'application.
- Garde-fou coût : au plus 1 appel de synchronisation par ouverture
  d'application et par ouverture de dossier, sans transfert de ce qui
  n'a pas changé.
- Garde-fou non-régression : l'usage solo hors ligne reste intact,
  enregistrement, transcription, chat, sans compte requis.

## Constraints & assumptions

- iOS d'abord, macOS dans la foulée. Android, Windows, Linux plus tard,
  mais rien ne doit fermer la porte.
- Le compte et le profil public de #86 sont la base d'identité ; le
  vocabulaire « thème » peut évoluer, le mot n'est pas figé.
- Un espace partagé exige une connexion ; l'usage solo n'en exige
  aucune et ne doit pas régresser.
- Le budget serveur est une contrainte de conception : la fraîcheur ne
  se paie pas en interrogation continue.
- L'app reste 100% Rust, une note reçue suit le pipeline existant
  d'indexation.

## Open questions

- Légalité de la suppression forcée des contributions d'un membre
  révoqué à son insu : à vérifier avant de câbler ce chemin.
- Expiry obligatoire de #86 : un espace d'équipe vit-il sans date de
  fin, ou hérite-t-il d'une expiration renouvelable ?
- Quota : nombre de membres, nombre de notes, taille par espace.
- Modération à l'échelle de l'espace : signalement et blocage de #86
  s'appliquent-ils tels quels ?
- Une note d'un membre reste-t-elle dans l'espace après son départ
  quand il ne demande pas son retrait ?
- Audio : une note vocale partagée transporte-t-elle son fichier son,
  ou seulement sa transcription ?
