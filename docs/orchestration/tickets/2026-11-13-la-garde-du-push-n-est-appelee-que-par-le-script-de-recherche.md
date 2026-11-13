# La garde du push n'est appelée que par le script de recherche ; les autres scripts de dernier pas ne la voient pas

- **État** : ouvert (13 novembre 2026, 22 h 30, orchestration).
- **Gravité** : règle de push — la décision de rejeu d'un lot dépend du script de chaque session.
- **Atteignable en service** : non (procédure de push seulement).
- **Touche rag3weaver** : non (tools/poste et la procédure).
- **Famille** : produit (la procédure de push et ses outils)
- **Pour** : codeparsers (auteur de la garde), recherche en relecture ; toute réattribution passe par l'orchestration.
- **Signalé par** : c7, à la lecture des pushes d'embarquement et d'ED du 13 novembre au soir.
- **Constat** : `tools/poste/push_garde.sh` (recopié en `~/.cache/rag3weaver-build/push_garde.sh` aux deux
  postes, 0e3d202a4) est le **script de dernier pas de recherche** (worktree, journaux, VERT, libs, code
  apporté, rebase, push, trois contrôles). Ses modes `--code-apporte <base> <neuf>` et `--derive <arbre>`
  sont appelables à part. Mais **`poste push` ne l'exécute pas** : rien dans `scripts/poste` ne l'appelle.
  Chaque session a son propre script de dernier pas (embarquement `pousser-spans.sh`, ED, tty, archi, 96,
  arbre principal…), avec sa propre détection du code apporté. Ce soir, deux pushes (embarquement 5d897d094,
  ED b7937d42d) n'ont imprimé ni « garde à jour » ni le code apporté par la garde : la décision de rejeu
  venait de leur propre détection, avec un verdict identique par chance.
- **Ce que la procédure dit à tort** (`docs/procedures/push-d-un-lot.md`, § « Ce qui compte comme du
  code ») : « la copie qu'exécute `poste push` est `~/.cache/rag3weaver-build/push_garde.sh` ». La copie
  fait autorité, mais rien ne l'exécute d'office.
- **Ce qu'il faut** :
  1. tout script de dernier pas appelle `~/.cache/rag3weaver-build/push_garde.sh --code-apporte <base testée> <master>`
     pour décider du rejeu, et `… --derive "$PWD"` après le rebase, sur l'arbre qui part ; aucune détection
     propre ; embarquement l'a déjà fait (22 h 25), les autres suivent avant leur prochain push ;
  2. la procédure nomme ces deux appels, mot pour mot, et cesse de dire que `poste push` exécute la garde ;
  3. un témoin : un dernier pas joué sur un dépôt jetable sans ces deux appels est refusé — la garde refuse
     de pousser si `--derive` n'a pas été appelé sur cet arbre dans la même minute, ou la relecture le lit ;
     au choix du lot.
- **À qui** : codeparsers (auteur de la garde), recherche en relecture, demain.
- **Pourquoi** : « vert est aucun rouge » tient sur la décision de rejeu ; six détections différentes ne
  font pas une règle.
