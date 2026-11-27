# Règles de publication

Ce dépôt est **public**. Tout ce qui y entre est visible de tous et le reste : fichiers, tout
l'historique Git (même un fichier supprimé ensuite), releases et leurs notes, journaux GitHub
Actions. Le catalogue de la branche `main` est en plus téléchargé par tous les launchers
installés. Un push sur `main`, un tag `vX.Y.Z` ou une release sont donc des **publications**.

## Ce qui peut être publié

- Le code, la documentation et les paquets du launcher.
- Dans le catalogue, uniquement les applications **publiques** : MIA
  (`MyAlfred-ai/mia`) et myBrief (`MyAlfred-ai/my-brief`), installées depuis leur dépôt public.
- Pour une application pas encore publiée (« Bientôt disponible ») : son nom, son accroche, son
  icône et sa couleur. Ni description, ni fonctionnalités, ni prérequis, ni nouveautés.
- Les coordonnées publiques de MyAlfred.ai : site `myalfred.ai`, `contact@myalfred.ai`, lien
  public de prise de rendez-vous.

## Ce qui ne doit jamais être publié

- **Secrets** : token, clé d'API, mot de passe, clé privée (dont la clé de signature du
  launcher), certificat, fichier `.env`, chaîne de connexion avec mot de passe.
- **Données clients** : nom de client ou de prospect, adresse e-mail, domaine, serveur, capture
  d'écran ou journal tiré d'une vraie installation.
- **Informations internes de MyAlfred.ai** : adresse de serveur, sous-domaine interne
  (tout `*.myalfred.ai` hors `myalfred.ai` et `www.myalfred.ai`), adresse IP privée, e-mail
  nominatif, dépôt privé, procédure interne, tarif non publié, détail d'une application non
  publiée.

Dans les exemples, tests et données de démonstration, utiliser des valeurs fictives :
`vous@entreprise.fr`, `mia.entreprise.fr`, `example.com`, `demo`.

## Vérification automatique

`scripts/verif-publication.mjs` (ou `npm run verif:publication`) bloque la publication si elle
trouve :

- un fichier interdit (`.env`, `.pem`, `.key`, `.p12`, base de données, dump…) ;
- un secret reconnaissable (tokens GitHub, clés Anthropic, OpenAI, AWS, Google, Slack, Stripe,
  clé privée, mot de passe dans une URL…) ;
- un sous-domaine `myalfred.ai` interne, un e-mail hors liste blanche, une IP privée, un dépôt
  `MyAlfred-ai/…` autre que `mia`, `my-brief` ou `myLauncher` ;
- dans le catalogue, une application publiée hors de MIA et myBrief, ou dont le dépôt n'est pas
  public sur GitHub, ou une application « Bientôt disponible » avec des détails ;
- un des mots de la liste privée (noms de clients, projets internes…), si elle est fournie.

Elle tourne :

- avant chaque `git push` (hook `.githooks/pre-push`, activé par `npm install`), sur les
  fichiers et les lignes ajoutées par les commits envoyés ;
- en CI, sur chaque pull request et chaque push sur `main`, sur les fichiers **et** sur les
  lignes ajoutées par les nouveaux commits (l'historique est public lui aussi) ;
- dans le workflow **Release**, avant toute construction : sans elle, pas de paquet ; le commit
  publié doit en plus être déjà sur `main`.

La liste privée de mots interdits vient du secret GitHub `PUBLICATION_MOTS_INTERDITS` (un mot
ou une expression par ligne) ; elle n'est jamais écrite dans le dépôt. Les journaux Actions
étant publics, le script n'affiche que le fichier, la ligne et la règle, jamais le texte
trouvé.

## Avant chaque publication

1. `npm run verif:publication` (ajouter `MOTS_INTERDITS` en local pour la liste privée).
2. Relire le diff : la vérification ne remplace pas la relecture (une donnée client n'a pas de
   forme reconnaissable).
3. Ne publier un tag ou une release qu'une fois la CI de `main` au vert.

## En cas de fuite

1. **Révoquer d'abord** le secret exposé (token, clé, mot de passe) : un contenu publié doit
   être considéré comme copié, même effacé aussitôt.
2. Retirer le contenu, puis prévenir l'équipe ; une réécriture de l'historique ne suffit pas
   (forks, caches, références des pull requests).
