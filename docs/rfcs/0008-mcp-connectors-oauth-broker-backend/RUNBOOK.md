# RUNBOOK - Connecteur Google Sheets (MCP) de bout en bout

But : que le chat de FlowFlow puisse agir sur un Google Sheet ("ajoute une ligne...").
Ce fichier est le mode d'emploi unique, prêt à suivre. Deux parties : un test rapide (prouver
que ca marche), puis la vraie installation (version utilisable dans le futur).

La chaine complete :

```
App FlowFlow  ->  backend (portier + OAuth)  ->  serveur Klavis Sheets  ->  Google
   chat            marketplace-flowflow            Docker self-host          ton tableur
```

---

## Etat actuel (lance par Claude, en local)

| Brique | Etat | Adresse |
|--------|------|---------|
| Backend (marketplace-flowflow) | tourne | http://localhost:8099 (healthz = ok) |
| Serveur Klavis Sheets (Docker, SKIP_OAUTH) | tourne | http://localhost:5050 (conteneur `ff-sheets`) |
| Token Google | MANQUANT | a fournir par toi (voir Partie A) |

Relancer si besoin :

```bash
# backend
MASTER_KEY=<hex32> GOOGLE_CLIENT_ID=placeholder GOOGLE_CLIENT_SECRET=placeholder \
GOOGLE_REDIRECT_URI=https://example.com/v1/connectors/google/callback \
DATABASE_URL='sqlite:/Users/mirkobozzetto/code/marketplace-flowflow/app.db?mode=rwc' PORT=8099 \
/Users/mirkobozzetto/code/marketplace-flowflow/target/debug/marketplace-flowflow

# serveur sheets (port 5050 car macOS occupe le 5000 avec AirPlay)
docker run -d --name ff-sheets -p 5050:5000 -e SKIP_OAUTH=true \
  ghcr.io/klavis-ai/google-sheets-mcp-server:latest
```

---

## PARTIE A - Test rapide (prouver qu'une ligne s'ecrit, ~5 min)

Ce test n'utilise PAS l'app ni le chat. Il prouve juste que la chaine token -> serveur Sheets ->
Google ecrit bien une ligne. Tout est deja pret sauf le token.

### Ton unique action : recuperer un token Google (3 clics)

1. Ouvre https://developers.google.com/oauthplayground
2. Dans le champ de gauche "Input your own scopes", colle exactement :
   `https://www.googleapis.com/auth/drive.file`
3. Clique **Authorize APIs**, connecte-toi avec TON Google, accepte.
4. Etape 2 (bouton bleu) : clique **Exchange authorization code for tokens**.
5. Copie la valeur **Access token** (commence par `ya29...`). Valide ~1 heure.

Donne ce token a Claude. Il lance :

```bash
GOOGLE_ACCESS_TOKEN=ya29.<ton-token> \
SHEETS_MCP_URL=http://127.0.0.1:5050 \
cargo run --manifest-path /Users/mirkobozzetto/code/marketplace-flowflow/Cargo.toml \
  --example connector_spike
```

Resultat attendu : un Google Sheet "FlowFlow CRM" est cree dans ton Drive avec une ligne dedans,
et le terminal affiche `SPIKE PASS`. Tu ouvres ton Google Drive, tu vois le tableur. Fin du test.

Pourquoi Claude ne peut pas faire l'etape token : se connecter a ton compte Google = tes
identifiants. Interdit pour l'assistant. Seul toi autorises Google.

---

## PARTIE B - La vraie version (utilisable dans le futur)

Ici le but est que, dans l'app, tu tapes dans le chat et ca ecrit dans ton Sheet. Ca demande un
backend en ligne (pas localhost) et une vraie app Google. Pas faisable en local en 5 min.

### B.1 - Variables d'environnement du backend (toutes requises pour booter)

| Variable | Role | Exemple |
|----------|------|---------|
| `MASTER_KEY` | chiffre les refresh tokens. `openssl rand -hex 32` | `0e17...` |
| `GOOGLE_CLIENT_ID` | ton app Google (voir B.2) | `xxx.apps.googleusercontent.com` |
| `GOOGLE_CLIENT_SECRET` | idem | `GOCSPX-...` |
| `GOOGLE_REDIRECT_URI` | ou Google renvoie apres login | `https://<ton-domaine>/v1/connectors/google/callback` |
| `SHEETS_MCP_URL` | adresse du serveur Klavis Sheets | `http://sheets:5000` (en compose) |
| `DATABASE_URL` | base SQLite | `sqlite:/data/app.db?mode=rwc` |
| `PORT` | port d'ecoute | `8099` (pour matcher l'app) |

### B.2 - Creer l'app Google (une seule fois, ton compte) - TON action

1. Google Cloud Console -> APIs & Services -> Library : active **Google Sheets API** + **Google Drive API**.
2. Credentials -> Create OAuth client ID -> type selon B.4.
3. Redirect URI = ton `GOOGLE_REDIRECT_URI`.
4. Recopie client id + secret dans les variables du backend.
5. Scope = `https://www.googleapis.com/auth/drive.file` (le backend cree le tableur, pas besoin de plus).

### B.3 - Mettre le backend en ligne (HTTPS)

`localhost` ne peut pas recevoir le retour de Google. Il faut un vrai domaine HTTPS.
Le `compose.yml` du repo lance deja backend + serveur Sheets ensemble. Suivre le README du backend
(section "Deploy on Dokploy"). Apres deploiement, dans l'app : Reglages -> Connections -> colle
l'URL HTTPS du backend, tape "Enregistrer le backend".

### B.4 - DECISION a trancher (le point reste ouvert du RFC) : le retour OAuth sur iPhone

L'iPhone capte le retour Google via un schema `flowflow://`. Or un client Google **Web** (celui qui
tient le secret) exige un retour http/https, pas un schema custom. Deux options :

- **Option A (recommandee)** : creer en plus un client Google **iOS** (public, PKCE, pas de secret) et
  garder le schema `flowflow://`. Le code device le fait deja.
- **Option B** : retour HTTPS via universal link (necessite iOS 17.4+ ; la cible actuelle est 16.0,
  donc bump de cible ou fallback).

Tant que ce point n'est pas tranche, le bouton "Connecter" cote app n'aboutit pas.

### B.5 - Te marquer premium (sinon 403)

Le premium est une allowlist de pubkeys cote backend (env `PREMIUM_PUBKEYS`), plus de flag en base.
Recupere la pubkey de ton appareil (base64, c'est le `device_id` envoye au handshake, visible dans les
logs backend a `/v1/auth/verify`), puis sur Dokploy :

```
PREMIUM_PUBKEYS=<pubkey_base64>[,<autre_pubkey>...]
```

Plusieurs appareils = liste separee par des virgules. Redeploie pour appliquer.

### B.6 - Tester dans le chat (sur iPhone)

Une fois connecte + premium, dans le chat tu tapes en langage normal, par exemple :

> "Ajoute dans mon CRM : Jean Dupont, plombier, 0470 12 34 56."

L'agent voit l'outil Google Sheets, l'appelle, et la ligne apparait dans le tableur "FlowFlow CRM".

---

## PARTIE C - Recommandation pour le futur

Le backend maison existe pour UNE raison : ne dependre d'aucun cloud Klavis. C'est aligne avec ta
privacy, mais c'est lourd (deploiement HTTPS + app Google + premium + decision iOS).

Deux chemins honnetes, choisis selon la priorite :

1. **Rester self-host (ta privacy)** : finir B.2 -> B.6. Le seul vrai travail = deployer le backend en
   HTTPS et trancher B.4 (Option A). Tout le reste est code et tourne deja.
2. **Aller vite (accepter le cloud Klavis pour CE connecteur)** : Klavis heberge gere l'OAuth pour toi
   (~30 min, https://www.klavis.ai/docs). Tu jettes le broker maison. Tes notes/audio restent locaux ;
   seul l'acces au connecteur passe par Klavis. Bon pour valider la fonction sans tout deployer.

Ne pas faire les deux a moitie. Trancher 1 ou 2 avant de continuer.
