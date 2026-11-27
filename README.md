# MyAlfred Launcher

Application de bureau (macOS, Windows, Linux) qui installe, lance et met à jour les applications
MyAlfred : **MIA** et **myBrief** (**myTrix** et **myBrain** bientôt), sur le poste de l'utilisateur ; les
nouvelles versions sont détectées et s'appliquent en un clic. L'installation sur un serveur, pour
toute une équipe, est proposée par MyAlfred.ai (page « Infos & contact » du launcher).

![Interface](docs/apercu.jpg)

## Installer le launcher

Télécharger le fichier de son ordinateur. Ces liens donnent toujours la dernière version publiée :

| Ordinateur | Fichier à télécharger |
|---|---|
| **Mac** (Apple Silicon et Intel) | [MyAlfred-Launcher-macOS.dmg](https://github.com/MyAlfred-ai/myLauncher/releases/latest/download/MyAlfred-Launcher-macOS.dmg) |
| **Windows** | [MyAlfred-Launcher-Windows.exe](https://github.com/MyAlfred-ai/myLauncher/releases/latest/download/MyAlfred-Launcher-Windows.exe) |
| **Linux** | [AppImage](https://github.com/MyAlfred-ai/myLauncher/releases/latest/download/MyAlfred-Launcher-Linux.AppImage) ou [paquet .deb](https://github.com/MyAlfred-ai/myLauncher/releases/latest/download/MyAlfred-Launcher-Linux.deb) |

- **Mac** : ouvrir le fichier téléchargé, glisser *MyAlfred Launcher* dans *Applications*, puis le
  lancer. Tant que le launcher n'est pas signé par Apple, macOS refuse le premier lancement :
  ouvrir *Réglages Système → Confidentialité et sécurité*, cliquer *Ouvrir quand même* en bas de
  la page et confirmer. À ne faire qu'une fois.
- **Windows** : double-cliquer le fichier téléchargé. Tant que le launcher n'est pas signé,
  Windows affiche « Windows a protégé votre ordinateur » : cliquer *Informations
  complémentaires*, puis *Exécuter quand même*.
- **Linux** : installer le paquet .deb, ou rendre l'AppImage exécutable (`chmod +x`) et la lancer.

Les versions suivantes s'installent ensuite depuis le launcher, en un clic.

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
scripts (`install.sh`, `kit/update.sh`…), comme le ferait un administrateur.

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

Tests en conditions réelles (lecture seule sur GitHub, clonage de MIA et myBrief sans token dans
un dossier temporaire ; avec `ML_TEST_PRIVATE_REPO=propriétaire/dépôt`, clonage d'un dépôt privé
avec le token) :

```bash
GITHUB_TOKEN=$(gh auth token) ML_TEST_DIR=$(mktemp -d) cargo test --manifest-path src-tauri/Cargo.toml -- --ignored live_
```

Avant tout push, tag ou release, voir [Publication](#publication).

## Publier une version

1. Mettre à jour la version dans `package.json`, `src-tauri/Cargo.toml` et `src-tauri/tauri.conf.json`.
2. Pousser un tag `vX.Y.Z` : le workflow **Release** construit les installeurs macOS (un seul
   paquet universel, Apple Silicon et Intel), Windows et Linux, les signe pour la mise à jour
   automatique et crée une release en brouillon avec `latest.json`. Il y ajoute une copie des
   installeurs sous un nom fixe (`MyAlfred-Launcher-macOS.dmg`, `-Windows.exe`,
   `-Linux.AppImage`, `-Linux.deb`).
3. Relire puis publier la release : les launchers installés la proposent, et les liens
   `releases/latest/download/…` de la section [Installer le launcher](#installer-le-launcher)
   servent aussitôt cette version.

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

## Applications publiées

Le launcher n'installe que des applications publiques, sans token, depuis leur dépôt public :
MIA ([MyAlfred-ai/mia](https://github.com/MyAlfred-ai/mia)) et myBrief
([MyAlfred-ai/my-brief](https://github.com/MyAlfred-ai/my-brief)). Les autres applications du
catalogue sont seulement annoncées (« Bientôt disponible », nom et accroche).

- **MIA** : avant chaque mise à jour, la base MySQL et les fichiers sont sauvegardés dans
  `~/backups/<dossier>` (3 sauvegardes conservées).
- **myBrief** : le launcher suit les versions publiées (releases `vX.Y.Z`, images signées
  `ghcr.io/myalfred-ai/mybrief/*`) et met à jour avec le kit du dépôt, `kit/update.sh`
  (sauvegarde MongoDB et `.env` dans `kit/sauvegardes`, signature vérifiée, retour arrière).

## Publication

Ce dépôt est **public** : son contenu, tout son historique Git, ses releases, ses notes de
version et les journaux GitHub Actions sont visibles de tous. Le catalogue de la branche `main`
est lu par tous les launchers installés.

Les règles sont dans [PUBLICATION.md](PUBLICATION.md) : rien d'autre que le launcher et les
versions publiques de MIA et myBrief ; aucune donnée client, aucune information interne de
MyAlfred.ai, aucun secret. Elles sont vérifiées automatiquement (CI sur chaque pull request et
chaque push sur `main`, puis avant chaque build de release) ; à lancer aussi avant de pousser :

```bash
npm run verif:publication
```

## Licence

[CC BY-NC-ND 4.0](LICENSE) : partage autorisé du launcher non modifié, avec mention de MyAlfred,
sans usage commercial ni version modifiée. Toute personne ou organisation peut en outre
l'installer et l'utiliser librement pour ses propres besoins. Les applications installées ont
leur propre licence (MIA et myBrief : AGPL-3.0).

## Limites connues

- **Windows** : les scripts des applications sont en bash ; le launcher utilise encore Git for
  Windows (Git Bash) pour les exécuter, avec Docker Desktop. L'installation guidée de Docker et
  le Git intégré sont à valider sur un vrai poste Windows.
- **myBrief en local** : expérimental (ports 80/443, certificat auto-signé sur `https://localhost`).
- **macOS** : sans signature Apple, le premier lancement doit être autorisé dans *Réglages Système
  → Confidentialité et sécurité → Ouvrir quand même* (depuis macOS 15, le clic droit → Ouvrir ne
  suffit plus). Les secrets `APPLE_*` (Developer ID et notarisation) suppriment cette étape.
- **Windows** : sans signature de code, SmartScreen avertit au premier lancement
  (*Informations complémentaires → Exécuter quand même*).
