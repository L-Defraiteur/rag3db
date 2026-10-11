# Ce qui manque à un agent dans cet IDE

*Ouverte le 11 octobre 2026 par la session codeparsers, à la demande de Lucie : « le vrai but
c'est qu'avec ce MCP vous vous sentiez déjà comme dans un IDE adapté à vous ; s'il y a des
choses à revoir, on reverra ; comme des malvoyants qui ont d'autres sens ».*

**C'est cette page qui décide des outils suivants.** Chaque session qui a le MCP du code
(`code-rag3db`, recette dans le rapport de la session codeparsers) y écrit une entrée **chaque
fois qu'elle retombe sur `grep`, une lecture par tranches ou un outil à côté**. Une entrée par
fois, même courte. Ne pas corriger les entrées des autres : en ajouter une.

## Comment écrire une entrée

```
### <date heure> — <session> — <la question, en une phrase>

- **La question posée** : ce que je voulais savoir, dans mes mots.
- **L'outil essayé** : `usages` / `impact` / `callees` / `search_code` / … et ses arguments.
- **Ce qu'il a rendu** : l'essentiel, copié (quelques lignes), ou « rien », ou l'erreur.
- **Ce que j'ai dû faire à côté** : `grep`, `read_file` par tranches, lire un autre fichier,
  deviner…
- **Ce qui manquait** : l'outil, le paramètre ou la réponse qui m'aurait évité ça.
```

## Les entrées

### 11 oct., 15 h 30 — codeparsers — monter le MCP du code pour rag3db (avant toute question)

Ce n'est pas une question d'agent, mais c'est le premier contact, et chaque point a coûté un
tour :

- **La question posée** : indexer `src/` et `test/` de rag3db par l'hôte.
- **L'outil essayé** : `estimate`, puis `index`, par un client MCP sur stdio.
- **Ce qu'il a rendu, et ce que j'ai dû faire à côté** :
  1. Un workspace en lecture seule (la racine est l'arbre principal, partagé) est refusé tant
     que le manifeste déclare `edit_file`, puis `run_command` : refus nommés et justes, mais il
     faut retirer ces outils à la main de la copie du gabarit. Il n'y a pas de gabarit
     « lecture seule ».
  2. Borner l'index à `src/` et `test/` : impossible. La source refuse les liens symboliques
     (« lien symbolique hors du projet », une garde voulue), et `generated.dirs` exclut un nom
     de dossier partout (il aurait pris `src/extension/`). J'ai indexé le dépôt entier
     (7 310 fichiers, 62 Mo).
  3. **L'hôte ne relit pas un manifeste modifié**, et aucune route ne ferme un backend : après
     chaque correction du manifeste, `estimate` rendait le premier état. J'ai dû changer de
     chemin de manifeste **et** de base (sinon deux backends sur la même base dans un même
     processus). L'ancien backend reste ouvert jusqu'au prochain redémarrage de l'hôte.
  4. `estimate` dit que l'index demandera environ 8 Gio de tampon, exactement le tampon donné
     (8 Gio) : à la limite.
- **Ce qui manquait** :
  - un gabarit `code` en lecture seule, prêt à l'emploi ;
  - des racines multiples, ou des chemins inclus, dans `workspace` ;
  - une relecture du manifeste par l'hôte (ou une route pour fermer un backend), avec un mot
    clair quand le backend servi n'est plus celui du fichier.
