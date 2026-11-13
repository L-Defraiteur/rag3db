# Un gabarit fourni masqué par un projet, puis démasqué, ne revient pas dans la bibliothèque

- **État** : ouvert. Trouvé par la session tty le 13 nov. 2026 pendant la passe du thème forêt, et reproduit sur master seul.
- **Famille** : produit
- **Gravité** : réponse fausse (une fiche de gabarit fournie manque au catalogue après une synchronisation).
- **Atteignable en service** : oui, par `sync_templates` d'un projet qui supprime un gabarit qu'il masquait.
- **Touche rag3weaver** : oui (le catalogue des gabarits).

## Ce que c'est

Un projet masque le gabarit fourni `entity/user` (un fichier `user.json` dans la couche du projet), puis supprime ce fichier. Après la synchronisation suivante, `sync_templates` rend bien `updated: ["entity/user"]`, mais la fiche `user` d'origine « builtin » n'est plus visible dans le catalogue. Elle devrait reprendre sa place, sous le même uuid.

## La recette

```bash
cd extension/rag3weaver
RAG3DB_LIB=rag3db-build-835d5298f80b832377a7caa80ee58d242e3ece23-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF \
CARGO_TARGET_DIR=<un target neuf> ./run_e2e.sh --features openai-llm,anthropic-llm --test e2e_catalogue_gabarits e2e_catalogue_gabarits
```

Rouge : 11 tests passent, 1 échoue, `la_synchronisation_alimente_la_couche_du_projet`, qui panique à `tests/e2e_catalogue_gabarits.rs:683` sur « user revenu à la bibliothèque » (étape 5 du test). La garde de `run_e2e.sh` relève aussi un résidu dans le dossier temporaire de la suite (`etat-temoins.*`, 4 Ko).

## La preuve qu'il ne vient pas d'un lot en cours

Le rouge est identique sur master seul, au commit `503a5ba2d`, avec la même lib et un target neuf (le worktree `/home/lucied/git_workspaces/cx-tty-temoin-master`, journal `~/.cache/rag3weaver-build/cx-tty-temoin-master.log`). On l'a d'abord vu sur la branche `cx-tty-foret`, qui n'ajoute que `templates/themes/foret.json` et ne touche aucun gabarit d'entité.

## Le témoin

Le test lui-même : `tests/e2e_catalogue_gabarits.rs`, `la_synchronisation_alimente_la_couche_du_projet`.

## La cause

Elle n'est pas cherchée. Pistes à lire :
- la reprise de la fiche fournie quand le fichier du projet disparaît (`sync_templates`) ;
- l'origine écrite sur la fiche ;
- `fiches_par_origine` côté test, peut-être après un changement récent du catalogue.

L'historique récent du dépôt montre plusieurs lots du 13 nov. sur les stockages et le journal (`d55b5264c`, `fd32269d8`). Rien n'est encore départagé.

## Ce qu'il faut pour le fermer

Le test vert sur master, puis la suite sans résidu dans son dossier temporaire, ou bien inscrite à `tests/e2e/suites_qui_laissent_des_residus.txt` si ce résidu est voulu.
