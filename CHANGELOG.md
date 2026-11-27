# Changelog

## [0.1.6] - 2026-11-27

### Ajouté
- Liens de téléchargement fixes, toujours vers la dernière version publiée
  (`releases/latest/download/MyAlfred-Launcher-macOS.dmg`, `-Windows.exe`, `-Linux.AppImage`,
  `-Linux.deb`) et guide d'installation pas à pas en tête du README.
- Liens de téléchargement en tête de chaque release.
- Licence CC BY-NC-ND 4.0, avec autorisation d'installer et d'utiliser librement le launcher.
- Règles de publication du dépôt public (PUBLICATION.md), vérifiées automatiquement avant chaque
  push, en CI et avant chaque release.

### Modifié
- Un seul installeur Mac, universel (Apple Silicon et Intel), au lieu de deux.
- myTrix et myBrain (« Bientôt disponible ») : seuls le nom et l'accroche sont affichés, sans
  présentation, prérequis, nouveautés ni dépôt source.

## [0.1.5] - 2026-11-27

### Modifié
- Token GitHub expiré ou révoqué : le launcher continue sans lui (MIA et myBrief se vérifient,
  s'installent et se mettent à jour normalement) au lieu d'échouer partout, et ne le renvoie plus.
  Un bandeau « Token GitHub refusé » mène aux réglages pour le remplacer ; les messages d'erreur
  (dépôt privé, téléchargement du code, limite de requêtes) disent aussi de le remplacer.

## [0.1.4] - 2026-11-09

### Ajouté
- Docker absent : le launcher télécharge Docker Desktop depuis le site officiel, vérifie sa
  signature et l'installe après acceptation de la licence (un mot de passe administrateur),
  puis le démarre et reprend l'installation de l'application.
- Docker arrêté : démarré automatiquement, à la vérification du poste comme avant chaque opération.
- Vérification de la mémoire allouée à Docker et de l'espace disque libre, avec des conseils clairs.

### Modifié
- Nouvelle icône du launcher (fusée et nœud papillon MyAlfred).
- Git n'est plus nécessaire sur le poste : le code des applications est récupéré par le Git
  intégré au launcher. Les installations existantes continuent de fonctionner.
- MIA vient de son dépôt public MyAlfred-ai/mia : plus besoin de token GitHub pour l'installer
  ou la mettre à jour. Les installations existantes passent au nouveau dépôt à la mise à jour
  suivante.
- MIA : la sauvegarde d'avant mise à jour (base MySQL et fichiers, dans `~/backups`, les 3
  dernières conservées) est faite par le launcher, le dépôt public n'ayant pas de `backup.sh`.
- myBrief vient de son dépôt public MyAlfred-ai/my-brief : plus besoin de token GitHub. Le
  launcher installe la dernière version publiée (images signées) et met à jour avec le kit du
  dépôt : sauvegarde, vérification des signatures, retour arrière en cas d'échec. Mot de passe
  provisoire de l'administrateur : 12 caractères au moins.
- Les applications « Bientôt disponible » (myTrix, myBrain) ne sont plus interrogées sur GitHub :
  sans token, le bandeau « Dépôts privés » ne s'affiche plus à tort.
- Le bandeau d'erreur de versions parle désormais de GitHub indisponible (hors ligne ou limite
  de requêtes) ; le token devient facultatif.

## [0.1.3] - 2026-10-19

### Ajouté
- Bloc « À prévoir » dans la présentation et dans l'assistant d'installation : ce qu'il faut
  préparer pour chaque application, avec liens vers les consoles.
- MIA : clé API d'IA indispensable (Anthropic, Mistral, OpenAI… ou IA locale via Ollama /
  llama.cpp) ; Google Cloud ou Microsoft Entra facultatifs mais conseillés (SSO, connecteurs
  Drive, Gmail, Outlook, OneDrive, SharePoint, Teams).
- myBrief : clé API OpenAI indispensable ; Google Cloud ou Microsoft Entra facultatifs mais
  conseillés (SSO, envoi par mail, dépôt sur Drive, OneDrive ou SharePoint).

## [0.1.2] - 2026-10-19

### Ajouté
- Mise à jour automatique du launcher : recherche au démarrage puis à intervalle régulier,
  notification et bandeau « Installer et redémarrer » (refusée pendant une opération en cours).
- Accueil : pour chaque application installée, adresse locale, voyant d'état, boutons
  « Se connecter » et Démarrer / Arrêter.
- Page « Infos & contact » : offres de MyAlfred.ai pour une installation sur serveur, prise de
  rendez-vous, formulaire et contact@myalfred.ai.
- myTrix et myBrain affichés « Bientôt disponible » : présentation et nouveautés visibles,
  installation pas encore ouverte.

### Modifié
- Les applications s'installent uniquement sur ce poste ; l'installation sur serveur est
  proposée par MyAlfred.ai.

### Supprimé
- Installation sur serveur par SSH et page « Serveurs ».

### Corrigé
- Les liens (application, documentation, GitHub) ne s'ouvraient pas dans l'app de bureau.
- Un port modifié dans le formulaire bloquait le bouton « Installer » sans message.
- Arrêter une application coupait la base de données sans lui laisser le temps de s'arrêter.
- MIA : port déjà pris détecté avant la construction, avec un message clair ; image Docker
  propre à chaque installation ; accès limité à ce poste dès le premier démarrage.
- myTrix : installation isolée d'un autre myTrix présent sur le poste.

## [0.1.1] - 2026-10-18

### Corrigé
- Installeurs macOS : signature ad hoc quand aucun certificat Apple n'est configuré
  (la 0.1.0 n'avait pas pu être construite pour macOS).

## [0.1.0] - 2026-10-18

### Ajouté
- Catalogue de MIA, myBrief, myTrix et myBrain, avec présentation, prérequis et nouveautés.
- Installation guidée sur ce poste ou sur un serveur Linux (SSH), avec vérification de Docker,
  Compose et Git, et suivi en direct (étapes, journal détaillé, annulation).
- Détection des nouvelles versions (releases, tags, fichier de version ou commit), notifications,
  « Tout mettre à jour » et mises à jour automatiques par installation.
- Démarrer, arrêter, redémarrer, journaux des conteneurs, désinstallation en gardant les données.
- Gestion des serveurs, token GitHub dans le trousseau du système, thème clair/sombre.
- Mise à jour signée du launcher et workflow de publication multi-plateformes.
