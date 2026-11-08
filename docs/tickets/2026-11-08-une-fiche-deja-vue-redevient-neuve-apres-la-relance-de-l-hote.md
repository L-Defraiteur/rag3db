# Une fiche déjà vue redevient neuve après la relance de l'hôte

- **État** : ouvert (8 novembre 2026), relevé par hooks-essai, demandé en ticket à part par c7.
- **Famille** : produit
- **Gravité** : bruit dans le contexte de l'agent. Si la cause est le hachage, chaque pose de G fait
  rendre de nouveau les fiches déjà vues de toutes les sessions branchées.
- **Atteignable en service** : oui, le hook UserPromptSubmit après toute relance de l'hôte sur un
  autre binaire.
- **Touche rag3weaver** : oui (`src/claude_code_hook.rs`, `empreinte_de_fiche` et `EtatDuRappel`).

## Ce que c'est

Le déjà-vu par session (`~/.local/state/rag3weaver/hooks/<session_id>.json`) garde pour chaque
fiche rendue son tour et son empreinte. `empreinte_de_fiche(nom, about)` hache le nom et le résumé
par `std::collections::hash_map::DefaultHasher`. À l'empreinte égale, une fiche est « déjà rendue » ;
sinon, elle revient « mise à jour ».

Session 494dbbc6 :
- 03 h 37, tour 106 : une fiche de bilan est rendue, et son empreinte est notée.
- 03 h 43 : G relance l'hôte sur le binaire eb154bf5f.
- 03 h 45, tour 107 : la même fiche est comptée neuve. Le journal
  (`~/.local/state/rag3weaver/hooks.log`) :

```
2026-11-08 03:45:11 rappel : exclu : « hooks-essai » (fiche propre) ; déjà rendu : 0, neuf : 2 ; 1 rendu, 1 au juge
```

L'état n'a pas été remis à zéro : la fiche y est encore, avec le tour 106. Son empreinte a donc
changé. Deux causes sont possibles, et rien ne les départage encore :

1. **Le résumé (`about`) de la fiche a été réécrit** entre les deux tours : le juge, ou un recalcul
   du L2 à la réouverture de la mémoire. Le comportement est alors voulu (« mise à jour »).
2. **`DefaultHasher` a rendu une autre valeur avec le nouveau binaire.** La bibliothèque standard
   ne garantit pas la stabilité de cet algorithme d'une version de Rust à l'autre. C'est alors un
   défaut du déjà-vu, à chaque pose.

## Ce qu'il faut pour le fermer

- **Départager** : relire le résumé de la fiche à ses deux révisions (`get_fiche`, `updated_at` et
  `revision`), et recalculer son empreinte avec les deux binaires sur le même `(nom, about)`.
- **Si c'est le hachage** : une empreinte explicite et stable, blake3 (déjà dans le crate) sur le
  nom et le résumé, en hexadécimal. Les états existants se relisent une fois : une ancienne
  empreinte se reconnaît à sa forme (16 chiffres hexadécimaux), et elle est recalculée plutôt que
  comparée.
- **Témoin** : l'empreinte d'un `(nom, about)` fixé est une constante écrite dans le test. Un état
  écrit avec l'ancienne forme ne rend pas « neuve » une fiche déjà vue.
