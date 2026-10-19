# Changelog

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
