# A1 sur master : les sondes lisent les morceaux de la forme de leur base

18 novembre 2026, arbre principal.

## Ce qui est poussé

Trois commits, poussés par la garde au « à toi » de c7 :

- **c399bc86a7** : le rouge ;
- **38da65ce87** : le correctif ;
- **d57c36968e** : la doc de `base()` à sa place.

Le ticket `2026-11-13-les-sondes-bancs-et-mesures-du-code-lisent-scope-chunk-en-dur.md` est fermé. Sa fermeture
dit ce que chaque commit fait. Le reste, la sonde des identifiants lucivy, est un ticket à part.

## Le sujet de 38da65ce87

Le sujet de 38da65ce87 dit **« NOT COMPILED, NOT PLAYED »**. C'était vrai le 16 novembre, quand il a été écrit,
juste avant l'arrêt de 10 h 5x. On ne réécrit pas un commit poussé : le sujet reste, et il est historique.

Le lot a été compilé et joué depuis. Son juge est la passe `arbre-sondes` du 18 novembre, 14 h 03, à luciepc :

- tête reposée sur 411be25cb2, branche `arbre-principal/scope-chunks-des-sondes-5` ;
- lib 59ac9c86bc, sous `RAG3WEAVER_MOTEUR_ANCIEN=1` ;
- témoin `e2e_banc_hnsw` 1/1 ;
- `check_features` 8/8 ;
- lib 2 614 ;
- témoins navigateur 45/45 ;
- 0 test non jugé.

Les trois contrôles après le push : status vide, aucun rebase en cours, HEAD = master = d57c36968e.

## Le rouge d'avant

La passe précédente, sur 58d115885c, a donné 44/45 aux témoins navigateur. Le test rouge était un témoin du chat
étranger au lot, avec quatre lourds en même temps à luciepc. Rien n'est parti. Ticket :
`docs/arbre-principal/tickets/2026-11-18-le-temoin-du-chat-qui-garde-ses-actions-apres-une-panne-rougit-sous-charge.md`.

## Non jugé

`e2e_lectures::la_recherche_rend_les_mots_pendant_un_create_de_l_index_vectoriel` n'est pas jugé ici. Le lot ne
joue aucune suite réduite (`SUITES=-`).

## Ensuite

- le ménage de la tablette ;
- `ViewState`, après le `Lag` de recherche.
