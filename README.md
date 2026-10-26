# MyAlfred Launcher

Application de bureau (macOS, Windows, Linux) qui installe, lance et met à jour les applications
MyAlfred : **MIA**, **myBrief**, **myTrix** et **myBrain**, sur le poste de l'utilisateur ; les
nouvelles versions sont détectées et s'appliquent en un clic. L'installation sur un serveur, pour
toute une équipe, est proposée par MyAlfred.ai (page « Infos & contact » du launcher).

![Interface](docs/apercu.jpg)

## Ce que fait le launcher

- **Catalogue des 4 applications** : présentation, prérequis, dernière version publiée, nouveautés.
- **Installation guidée sur ce poste** : seul Docker est nécessaire. S'il manque, le launcher
  télécharge Docker Desktop depuis le site officiel, vérifie sa signature et l'installe (un mot de
  passe administrateur, après acceptation de la licence Docker) ; s'il est arrêté, il le démarre.
  Mémoire de Docker et espace disque sont vérifiés. Puis formulaire de configuration propre à
  chaque application et suivi en direct (étapes et journal détaillé).
- **Mises à jour** : vérification au démarrage puis à intervalle régulier, notification système,
  « Tout mettre à jour », mises à jour automatiques activables par installation. Une sauvegarde
  est faite avant quand l'application le prévoit.
- **Exploitation** : se connecter, démarrer, arrêter (depuis l'accueil ou la fiche de l'application),
  redémarrer, journaux des conteneurs, désinstaller (les données sont conservées).
- **Infos & contact** : offres de MyAlfred.ai pour une installation sur serveur (installation,
  hébergement en Europe, maintenance, formation), prise de rendez-vous, formulaire et e-mail.
  Les liens sont dans [src/lib/contact.ts](src/lib/contact.ts).
- **Mise à jour du launcher lui-même** : recherchée au démarrage puis à intervalle régulier,
  notification et bandeau « Installer et redémarrer » ; paquets signés, vérifiés avant
  installation ; refusée tant qu'une installation ou une mise à jour d'application est en cours.
- Interface en français, thème clair/sombre, utilisable de 380 px à plein écran.

## Fonctionnement

Chaque application MyAlfred est une application web auto-hébergée sous Docker. Le launcher ne
réinvente pas leur installation : il écrit leur configuration (`.env`) puis appelle leurs propres
scripts (`install.sh`, `./mytrix installer`…), comme le ferait un administrateur.

```
Interface (Svelte 5)  ──invoke──▶  Cœur Rust (Tauri 2)
                                     ├─ catalog/apps.json : description de chaque application
                                     ├─ GitHub API : versions, nouveautés, catalogue à jour
                                     ├─ Git intégré (libgit2) : code des applications
                                     └─ scripts bash générés ──▶ bash -s (ce poste)
```

- **Catalogue** ([catalog/apps.json](catalog/apps.json)) : dépôt, source de la version, champs du
  formulaire, étapes d'installation et de mise à jour en shell. Une copie est intégrée au launcher ;
  une version plus récente est téléchargée depuis ce dépôt (branche `main`) à chaque vérification.
  On peut donc corriger une procédure sans republier le launcher.
- **Versions** : selon l'application, dernière release, plus haut tag, fichier `VERSION` /
  `src/version.json`, ou dernier commit de `main`.
- **Exécution** : une opération enchaîne des scripts bash et la récupération du code (étapes `@sync`
  du catalogue), faite par le Git intégré au launcher : `git` n'a pas à être installé sur le poste.
  Les scripts sont envoyés sur l'entrée standard de `bash` ; les secrets n'apparaissent jamais dans
  une ligne de commande. Si `git` est absent, `git rev-parse HEAD` (utilisé par certains scripts
  des applications) répond avec le commit connu du launcher (`$ML_COMMIT`).
- **Docker** : avant chaque opération, Docker Desktop (ou OrbStack, Rancher Desktop, Podman
  Desktop) est démarré s'il ne répond pas.

### Format d'une étape

```json
{ "label": "Configuration", "run": "ml_dotenv DOMAIN \"$DOMAIN\"\nml_compose up -d" }
```

`run` s'exécute dans le dossier de l'application (`workdir` éventuel), sous `bash -Eeuo pipefail`.
La valeur spéciale `@sync` télécharge le code à la version visée. Les champs du formulaire sont
disponibles comme variables shell (non exportées). Fonctions utiles :

| Fonction | Rôle |
|---|---|
| `ml_dotenv CLÉ valeur [fichier]` | écrit une variable dans un `.env` en gardant les autres lignes |
| `ml_dotenv_default CLÉ valeur [fichier]` | idem, seulement si absente : un secret n'est jamais régénéré |
| `ml_rand_hex N`, `ml_rand_b64 N` | secrets aléatoires |
| `ml_compose …` | `docker compose` dans le dossier de l'application |
| `ml_pull` | `docker compose pull`, avec connexion à ghcr.io si besoin |
| `ml_wait SECONDES commande…` | attend qu'une commande réussisse |
| `ml_http_ok URL`, `ml_container_http SERVICE URL` | sondes HTTP (hôte ou conteneur) |
| `ml_info`, `ml_warn`, `ml_fail` | messages affichés dans le suivi |

## Développement

Prérequis : Node 22+, Rust (la version est fixée par `rust-toolchain.toml`), et sous Linux les
bibliothèques WebKitGTK de Tauri.

```bash
npm install
npm run tauri dev      # application de bureau
npm run dev            # interface seule dans un navigateur, avec des données de démonstration
npm test               # tests de l'interface
npm run check          # vérification des types
cargo test --manifest-path src-tauri/Cargo.toml   # tests du cœur (dont exécution réelle sous bash 3.2)
```

Tests en conditions réelles (lecture seule sur GitHub, clonage d'un dépôt privé et de MIA,
publique, sans token, dans un dossier temporaire) :

```bash
GITHUB_TOKEN=$(gh auth token) ML_TEST_DIR=$(mktemp -d) cargo test --manifest-path src-tauri/Cargo.toml -- --ignored live_
```

## Publier une version

1. Mettre à jour la version dans `package.json`, `src-tauri/Cargo.toml` et `src-tauri/tauri.conf.json`.
2. Pousser un tag `vX.Y.Z` : le workflow **Release** construit les installeurs macOS (Apple Silicon
   et Intel), Windows et Linux, les signe pour la mise à jour automatique et crée une release en
   brouillon avec `latest.json`.
3. Relire puis publier la release : les launchers installés la proposent.

Secrets GitHub requis : `TAURI_SIGNING_PRIVATE_KEY` (et `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` si la
clé en a un). La clé privée de signature n'est **pas** dans le dépôt ; la clé publique est dans
`src-tauri/tauri.conf.json`. Pour une diffusion publique sur macOS, ajouter aussi les secrets
`APPLE_*` (certificat Developer ID et notarisation).

## Sécurité

- Le token GitHub est rangé dans le trousseau du système (Keychain, Gestionnaire d'identifiants
  Windows, Secret Service) et n'est envoyé qu'aux adresses GitHub.
- Pour cloner, le token est remis au Git intégré seulement quand GitHub le demande : il n'est jamais
  écrit dans `.git/config`.
- L'installeur de Docker Desktop est téléchargé depuis desktop.docker.com et n'est exécuté
  qu'après vérification de sa signature (équipe Apple `9BNSXJN65R` de Docker Inc. sur macOS,
  signature Authenticode « Docker Inc » sur Windows), vérification refaite avec les droits
  administrateur juste avant l'installation.
- Les mots de passe saisis à l'installation ne sont pas conservés par le launcher ; ils vont
  seulement dans la configuration de l'application.
- Désinstaller arrête et supprime les conteneurs mais garde volumes, `.env` et dossier.

## Passage des dépôts en public

Rien à changer dans le code : sans token, le launcher lit les versions publiques et clone en
HTTPS anonyme. La mise à jour du launcher fonctionne dès que les releases de ce dépôt sont publiques.

MIA est publique ([MyAlfred-ai/mia](https://github.com/MyAlfred-ai/mia)) : elle s'installe et se
met à jour sans token. Les installations faites depuis l'ancien dépôt privé `myLLM` basculent
d'elles-mêmes à la mise à jour suivante. Tant que ce dépôt-ci reste privé, un launcher sans token
ne lit pas le catalogue distant et garde celui qu'il embarque : publier une version du launcher
pour que tous profitent du changement. Le dépôt public n'ayant pas de `backup.sh`, la sauvegarde
d'avant mise à jour (base MySQL et fichiers, dans `~/backups/<dossier>`, 3 conservées) est
écrite dans le catalogue.

## Limites connues

- **Windows** : les scripts des applications sont en bash ; le launcher utilise encore Git for
  Windows (Git Bash) pour les exécuter, avec Docker Desktop. L'installation guidée de Docker et
  le Git intégré sont à valider sur un vrai poste Windows.
- **myBrief en local** : expérimental (ports 80/443, certificat auto-signé sur `https://localhost`).
- **macOS** : sans signature Apple, le premier lancement demande un clic droit → Ouvrir.
