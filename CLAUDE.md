# Consignes pour les agents

## Dépôt public : règles de publication

Ce dépôt est public. Lire [PUBLICATION.md](PUBLICATION.md) avant tout commit, push, tag,
release ou paquet. En bref :

- Ne publier que le launcher et les versions publiques de **MIA** et **myBrief**.
- **Jamais** de secret (token, clé, mot de passe, `.env`, clé de signature), de donnée client,
  ni d'information interne de MyAlfred.ai (serveurs, sous-domaines internes, e-mails
  nominatifs, dépôts privés, détails d'applications non publiées).
- Applications « Bientôt disponible » : nom, accroche, icône, couleur, rien de plus.
- Exemples et tests : valeurs fictives (`vous@entreprise.fr`, `mia.entreprise.fr`, `demo`).
- Lancer `npm run verif:publication` avant tout commit destiné à `main`, tout tag et toute
  release ; ne jamais contourner ni affaiblir cette vérification. Elle tourne aussi en CI et
  bloque le workflow Release.
- Ne jamais afficher dans un journal (CI comprise, journaux publics) le texte d'un secret ou d'un
  mot interdit trouvé.

## Développement

Voir [README.md](README.md) : commandes, tests, publication d'une version.
