# Les bilans du juge remontent au rappel sur des mots courants

- **État** : ouvert (8 novembre 2026), relevé par hooks-essai sur deux tours de suite, puis demandé
  en ticket par c7.
- **Famille** : produit
- **Gravité** : bruit dans le contexte de l'agent. Un compte rendu de run, sans rapport avec le
  message, est rendu en tête du tour.
- **Atteignable en service** : oui, le hook UserPromptSubmit de toute session Claude Code branchée
  sur la mémoire longue.
- **Touche rag3weaver** : oui (`src/claude_code_hook.rs`, le rappel des fiches ; le nom que le juge
  donne à une fiche de bilan).

## Ce que c'est

Aux deux messages de G sur la coupure de l'hôte (8 novembre, 03 h 37 et 03 h 45), le hook a rendu
une fiche « bilan : tâche réelle, bornée, dans ce worktree du dépôt rag3db … » : le bilan d'une tâche
sur les `*.undo.json` laissés dans /tmp, écrit par le juge (`via: juge`), sans rapport avec les deux
messages. Le journal (`~/.local/state/rag3weaver/hooks.log`) :

```
2026-11-08 03:45:11 rappel : requête « optimiseur hooks-essai 7970 est relancé sur et la mémoire rouverte » (10 mots, tirée d'un prompt de 253 caractères)
2026-11-08 03:45:11 rappel : « hooks-essai » plein texte 12.00, dense 0.51, « bilan : tâche réelle, bornée, dans ce worktree du dépôt rag3db (extension/rag3weaver). ticket : docs/tickets/2026-10-04-journaux-d-annulation-laisses-dans-tmp.md — … » …
2026-11-08 03:45:11 rappel : exclu : « hooks-essai » (fiche propre) ; déjà rendu : 0, neuf : 2 ; 1 rendu, 1 au juge
```

Quatre choses s'y lisent :

1. **Le nom d'une fiche de bilan est le prompt entier du run** : 2 264 et 1 811 caractères pour les
   deux fiches de ce relevé. Le plein texte touche donc un bilan sur n'importe quel mot courant du
   dépôt (hôte, worktree, rag3db, mémoire). Un bilan est le compte rendu d'un run, pas un savoir à
   rappeler.
2. **Deux runs de la même consigne donnent deux fiches dont les 600 premiers caractères sont
   identiques.** Le bloc borné les montre pareilles : « la même fiche deux fois » était en fait
   deux fiches. L'état du déjà-vu de la session
   (`~/.local/state/rag3weaver/hooks/<session>.json`) les a notées sous leurs deux noms, au tour
   106 (« … tout ce qui n'est pas dans la boîte est perdu ») et au tour 107 (« … pose la question
   dans la boîte et attends ma réponse »).
3. **Le déjà-vu a pourtant compté la fiche du tour 106 comme neuve au tour 107** (« déjà rendu :
   0, neuf : 2 ») : son empreinte (`empreinte_de_fiche`, nom et résumé, `DefaultHasher`) avait
   changé entre 03 h 37 et 03 h 45. L'hôte a été relancé à 03 h 43 sur un autre binaire
   (eb154bf5f). La cause n'est pas départagée :
   - le résumé de la fiche a-t-il été réécrit ?
   - ou le hachage a-t-il changé avec le binaire ? `DefaultHasher` n'est pas garanti stable d'une
     version de Rust à l'autre.

   À la fin du tour 107, les deux fiches étaient déjà vues (03 h 46 : « déjà rendu : 2, neuf : 0 ;
   silence »).
4. Le second candidat est parti « au juge » ; sans modèle dans l'environnement du hook, il a été
   tu (« [fond] sans juge (aucun modèle dans l'environnement du hook) : 0 rendu, 1 tus »).

## Ce qu'il faut pour le fermer (propositions, à trancher)

- **A. Écarter les bilans du rappel en tête de tour** : une fiche écrite par le juge (son L1 vient
  de `via: juge`, ou le nom commence par « bilan : ») n'est pas candidate au rappel de
  UserPromptSubmit. Elle reste lisible par `get_fiche`, et le juge différé la compare à la session
  suivante, ce qui est son usage.
- **B. Ou ne les faire porter que par le juge différé** : un bilan touché au plein texte va au juge,
  jamais au rendu direct. Il ne paraît que si le juge le dit en rapport.
- **Dans les deux cas** :
  - un nom court pour une fiche de bilan (un titre et l'empreinte de la consigne, pas le prompt
    entier), pour que deux runs ne se ressemblent plus au bornage ;
  - une empreinte du déjà-vu stable d'un binaire à l'autre (un hachage explicite, blake3 par
    exemple, déjà dans le crate), pour que la relance de l'hôte ne rende pas « neuve » une fiche
    déjà vue.

Témoin : le rappel d'un prompt qui contient « worktree » et « rag3db », avec une fiche de bilan
dans la mémoire du banc. Le bilan n'est pas rendu (A), ou il va au juge (B). La même fiche, rendue
une fois, reste « déjà vue » après un changement de binaire, ce que simule une empreinte relue
depuis l'état.
