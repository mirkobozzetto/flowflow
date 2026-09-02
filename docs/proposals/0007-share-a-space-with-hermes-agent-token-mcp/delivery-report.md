# Accès d’un espace FlowFlow pour Hermes

Mise à jour : 2 septembre 2026

## État actuel

Le socle du backend est terminé jusqu’à T04. Les routes réservées au
propriétaire et les limites de sécurité sont encore en cours.

La fonctionnalité n’est pas encore prête pour un test réel avec Hermes.
Le blocage actuel est simple : le serveur MCP n’existe pas encore et le
backend modifié n’est pas déployé.

Hermes utilisera son client MCP HTTP natif. Aucun connecteur spécial n’est
nécessaire. Le VPS aura seulement besoin de l’URL du backend FlowFlow et d’un
jeton `mcps_` limité à un espace.

## Travail terminé

| Tâche | Résultat | Commit |
|---|---|---|
| T01 | Extraction des routes espace vers un cœur partagé | `255c4df` |
| T02 | Identités, jetons, audit et auteurs agents en base | `2b90238` |
| T03 | Authentification uniforme des jetons agents | `f749597` |
| T04 | Permissions agent et isolation stricte par espace | `b82140a` |

## Tests déjà exécutés

| Portée | Commande ou contrôle | Résultat |
|---|---|---|
| T01 | `cargo test --test spaces_test` | 25 tests réussis |
| T02 | Migration peuplée et contraintes d’intégrité | Réussi |
| T03 | Jetons absents, invalides, expirés ou révoqués | Réponse 404 |
| T04 | `cargo test --test spaces_perm_test` | 12 tests réussis |
| T04 | `cargo test --test spaces_test` | 25 tests réussis |
| T04 | `cargo fmt --check` | Réussi |
| T04 | Diagnostic Rust Analyzer sur quatre fichiers | Aucune erreur |

## Ce qui reste réellement à faire

1. T05 : créer, lister, régénérer et révoquer un agent depuis le backend.
2. T06 : limiter chaque agent à 60 écritures par minute et 500 dossiers.
3. T07 : exposer les neuf outils MCP sur `/v1/mcp-spaces`.
4. T08 : tester écriture MCP, pull appareil, accusé et révocation.
5. T09 : documenter l’exploitation et la configuration Hermes.
6. T10-T11 : ajouter le client et le panneau agent dans l’app FlowFlow.
7. T12 : installer et vérifier l’app sur l’iPhone physique.
8. T13 : vérifier depuis le VPS que le backend répond sur `/healthz`.
9. T14 : configurer MCP et le skill `flowflow-spaces` sur le VPS.
10. T15 : planifier puis prouver une vraie exécution quotidienne.

## Pourquoi cela prend du temps

Ce n’est pas un simple branchement de configuration. Il faut construire la
chaîne complète sans donner à Hermes les droits d’un utilisateur humain :

- identité d’agent durable ;
- jeton secret stocké uniquement sous forme de hash ;
- permissions limitées à un espace ;
- serveur MCP avec neuf outils ;
- interface iPhone pour créer et révoquer l’accès ;
- déploiement du backend avant toute connexion réelle ;
- configuration et preuve d’exécution sur le VPS.

Rien venant de Mirko ne bloque T05 à T11. Le premier besoin externe arrivera
au déploiement : l’URL effective du backend doit être joignable depuis le VPS.
L’espace à partager et le jeton seront ensuite choisis depuis l’app.

## Configuration prévue sur le VPS

```yaml
mcp_servers:
  flowflow_<espace>:
    url: https://<backend>/v1/mcp-spaces
    headers:
      Authorization: "Bearer ${FLOWFLOW_TOKEN_<ESPACE>}"
    timeout: 30
```

Le jeton restera dans `~/.hermes/.env` sur le VPS. Hermes ne recevra ni mot de
passe FlowFlow, ni secret d’appareil, ni information concernant le téléphone.

Tailscale est nécessaire uniquement si le backend est privé dans le tailnet.
Un backend HTTPS public demande seulement une sortie HTTPS depuis le VPS.

## Limites de sécurité

- Un jeton donne accès à un seul espace.
- Le scope est `read` ou `read_write`.
- Le backend stocke uniquement le hash du jeton.
- La rotation et la révocation invalident les anciens accès.
- Un agent suit les règles d’un membre et modifie seulement ses notes.
- L’audit conserve des métadonnées, jamais le contenu ni les en-têtes.
- Le cron exposera uniquement le serveur MCP FlowFlow choisi.

## Condition avant le premier test Hermes

Le backend doit être terminé et déployé. L’app créera ensuite l’agent et
montrera son jeton une seule fois. Le contrôle `/healthz` depuis le VPS devra
répondre HTTP 200 avant d’ajouter ce jeton à Hermes.
