---
feature: Import audio dans une note pour transcription
slug: audio-import-transcription
type: prd
status: ready
stepsCompleted: [0, 1, 2, 3, 4]
---

# PRD: Import audio dans une note pour transcription

## Problem statement

FlowFlow ne sait transcrire que ce qu'il capte **en direct au micro**. Tout audio
déjà enregistré ailleurs est inaccessible: un mémo d'un dictaphone physique, un fichier
Voice Memos iPhone, l'enregistrement d'une réunion fait par une autre app.

Aujourd'hui, le seul contournement pour transcrire un fichier existant est de le **rejouer
à voix haute devant le micro** de FlowFlow: lent (temps réel), dégradé (perte de qualité,
bruit ambiant), et impossible pour une réunion de plusieurs heures.

C'est un manque direct sur la promesse du produit (transformer la parole en notes
exploitables): la matière première de l'utilisateur, ses enregistrements, reste piégée
hors de l'app. La douleur est forte pour les réunions longues déjà capturées, qu'on ne
peut ni importer ni transcrire.

## Goals

- Permettre d'**importer un fichier audio existant** dans une note depuis l'app, sans
  ré-enregistrement, et obtenir sa **transcription texte** dans la note.
- Couvrir les formats réels des dictaphones et téléphones (m4a/AAC, mp3, wav, caf).
- Supporter des fichiers **longs (plusieurs heures)** — réunions, conférences — sans
  échec dû à une limite de durée trop courte.
- Lancer la transcription en **arrière-plan**: l'utilisateur garde l'app utilisable
  pendant le traitement.
- **Détecter la langue automatiquement** (l'audio importé n'est pas forcément en français).
- Ne **jamais abîmer la note** en cas d'échec: message clair + possibilité de relancer.
- Réutiliser le pipeline existant (transcription, ajout au contenu de la note, indexation
  pour la recherche sémantique) pour que l'audio importé devienne une note de plein droit.

## Non-goals / Out-of-scope

- Pas d'import **multi-fichiers** en une fois (un seul fichier par import dans cette version).
- Pas de conservation du **fichier audio** importé: on transcrit puis on jette le fichier
  (pas de relecture audio de l'import). L'audio jouable reste réservé aux enregistrements live.
- Pas d'**extraction audio depuis une vidéo** (mov/mp4) dans cette version.
- Pas de **sélecteur de langue manuel** (l'auto-détection suffit; pas d'UI de choix).
- Pas de transcription **offline** (le service de transcription reste en réseau).
- Pas de **diarisation** (séparation des locuteurs) ni d'horodatage par segment.
- Pas d'édition audio (découpe, nettoyage) avant transcription.
- Pas de refonte de l'enregistrement live; seul le **plafond de durée partagé** (timeout)
  est étendu pour bénéficier aussi au live (voir Goals/Contraintes).

## User stories

1. **Import depuis une note.** En tant qu'utilisateur, je veux importer un fichier audio
   depuis le menu d'une note, afin d'en récupérer le contenu en texte sans le ré-enregistrer.
2. **Formats dictaphone.** En tant qu'utilisateur d'un dictaphone ou de Voice Memos, je veux
   que mes fichiers (m4a, mp3, wav, caf) soient acceptés, afin de ne pas avoir à les convertir.
3. **Réunion longue.** En tant qu'utilisateur, je veux importer une réunion de plusieurs
   heures, afin d'en obtenir la transcription complète sans échec de durée.
4. **Travail en arrière-plan.** En tant qu'utilisateur, je veux continuer à utiliser l'app
   pendant qu'une longue transcription tourne, afin de ne pas rester bloqué à attendre.
5. **Langue automatique.** En tant qu'utilisateur, je veux que la langue soit détectée
   automatiquement, afin d'importer aussi des audios non francophones.
6. **Échec sans dégât.** En tant qu'utilisateur, je veux qu'un import raté ne casse jamais
   ma note et me laisse relancer, afin de réessayer sans rien perdre.

## Acceptance criteria

**Story 1 (import depuis une note)**
- Given une note ouverte, When j'ouvre son menu, Then une action "Importer un audio" est
  disponible, à côté de l'import de documents.
- Given je choisis un fichier audio valide, When l'import se lance, Then la transcription
  démarre et, une fois prête, son texte est ajouté au contenu de la note.
- Given la transcription aboutie, Then la note est indexée pour la recherche sémantique
  comme une note normale (le contenu importé devient cherchable et utilisable par le chat).

**Story 2 (formats dictaphone)**
- Given le sélecteur de fichiers, When je parcours mes fichiers, Then seuls les formats
  audio supportés (m4a/AAC, mp3, wav, caf) sont sélectionnables.
- Given un fichier d'un format non supporté, When je tente l'import, Then il est refusé
  avec un message clair, sans toucher à la note.

**Story 3 (réunion longue)**
- Given un fichier audio de plusieurs heures, When je l'importe, Then la transcription
  va jusqu'au bout sans échouer à cause d'une limite de durée.
- Given une transcription longue en cours, When elle progresse, Then une progression
  visible indique que le traitement avance (pas d'impression de blocage).

**Story 4 (arrière-plan)**
- Given une transcription lancée, When je quitte la note ou navigue ailleurs dans l'app,
  Then l'app reste utilisable et la transcription continue.
- Given je reviens sur la note quand c'est prêt, Then le texte transcrit y est bien présent
  (le résultat n'est pas perdu par la navigation).

**Story 5 (langue automatique)**
- Given un audio importé dans une langue quelconque, When il est transcrit, Then la langue
  est détectée automatiquement (aucune sélection manuelle requise).

**Story 6 (échec sans dégât)**
- Given un échec (délai dépassé, format refusé, clé API absente, consentement IA non donné),
  When l'import échoue, Then la note reste intacte (aucun texte partiel inséré) et un message
  clair explique la cause.
- Given un import échoué, When je le relance, Then je peux réessayer sans re-créer la note.
- Given pas de clé Soniox ou pas de consentement IA, When je tente l'import, Then l'action
  est refusée tôt avec un message indiquant quoi configurer.

## Success metrics

- 100% des formats ciblés (m4a/AAC, mp3, wav, caf) sont importés et transcrits avec succès
  sur un jeu de fichiers de test représentatif.
- 0 cas de note corrompue ou de texte partiel inséré lors d'un import qui échoue.
- Un fichier de **≥ 2 heures** est transcrit jusqu'au bout sans échec de durée (cible:
  pas de timeout prématuré; valeur exacte de la borne haute à confirmer — voir Open questions).
- Pendant une transcription en cours, l'app reste **interactive** (navigation possible,
  aucune fenêtre bloquante).
- ≥ 90% des imports réussis aboutissent à un texte ajouté à la note et indexé pour la
  recherche, vérifiable en retrouvant le contenu via le chat/la recherche.

## Constraints & assumptions

- iOS uniquement, 100% Rust/Dioxus, cohérent avec l'architecture existante.
- Sélection de fichiers via le sélecteur de fichiers natif iOS déjà en place (Files, iCloud
  Drive, emplacements montés). Un seul fichier à la fois.
- Transcription via le service Soniox déjà intégré; **réseau requis**; clé Soniox + consentement
  IA requis (mêmes prérequis que l'enregistrement live).
- La limite de durée actuelle de la transcription est trop courte pour le multi-heures et
  doit être **étendue**; ce relèvement est **partagé** avec l'enregistrement live (même
  chemin de transcription) et bénéficie donc aux deux.
- Le fichier importé n'est **pas conservé** après transcription (décision produit: transcription
  seule). Seul le texte est gardé dans la note.
- Langue **auto-détectée** (on n'impose plus le français strict pour ce flux d'import).
- Le résultat de transcription doit survivre à la navigation pendant le traitement en
  arrière-plan (ne pas dépendre du fait que la note reste à l'écran).

## Open questions

- Borne haute exacte de durée/taille à supporter (ex: jusqu'à combien d'heures / quelle taille
  de fichier vise-t-on pour la métrique « va jusqu'au bout » ?).
- Auto-détection de langue: la garder aussi pour l'**enregistrement live** (aujourd'hui FR strict),
  ou la limiter au flux d'import pour ne pas changer le comportement live ?
- Feedback de progression d'une longue transcription: simple indicateur « en cours » suffisant,
  ou faut-il une progression chiffrée (pourcentage / temps écoulé) ?
- Notification quand une transcription d'arrière-plan se termine alors que l'utilisateur est
  ailleurs dans l'app: signal visuel discret, ou rien (il le voit en revenant sur la note) ?
- Comportement si l'app passe en arrière-plan iOS (écran verrouillé) pendant une longue
  transcription: à valider au regard des limites d'exécution iOS (lié au PRD `lan-serve`/agentic).
