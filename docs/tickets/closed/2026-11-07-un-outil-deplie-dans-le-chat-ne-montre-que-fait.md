# Un outil déplié dans le chat ne montre que « fait »

- **État** : corrigé (7 novembre 2026, hooks-essai, lot « le chat tient l'ordre ») : `ToolCallFinished` porte `result_excerpt` (4 Kio), puis le résultat entier se relit dans la session à la fin du tour ; l'outil déplié montre son heure, sa durée et son résultat ; témoin navigateur
- **Gravité** : réponse fausse à l'écran (le résultat d'un outil existe, la page ne le montre pas).
- **Atteignable en service** : oui, tout tour du chat qui appelle un outil (IDE posé à 17 h 22).
- **Touche rag3weaver** : oui (`ui/ide/src/views/Chat.tsx`, l'événement `ToolCallFinished`).
- **Famille** : produit

## Ce que c'est

Lucie : « je n'ai que ça d'affiché quand j'expand les outils : `search_code` /
`{"options":{"limit":5},"query":"project description overview database"}` / `fait` ».

## La cause (lue dans le code)

Pendant le flux, le résultat d'un outil vient de `ToolCallFinished`, qui ne porte pas le résultat :
la page écrit `fait` (`Chat.tsx`, `t.result ?? "fait"`). Le résultat entier est dans le tour `tool`
de la session (et dans le journal du run), mais la page ne le relit qu'au rechargement
(`fromTurns`) ; la réponse du tour (`sendChat`) ne le rend pas non plus.

## Ce qu'il faut pour le fermer

Un appel déplié montre le nom, les arguments **et le résultat rendu** : pour `search_code`, les
résultats avec leur chemin, le compte, la durée, `meta.warnings` et la source des surlignages ;
pour un outil qui lit, l'extrait ; pour une erreur, le message. Le résultat vient de la session ou
du journal du run, où il est déjà ; s'il n'y est pas, c'est un défaut de l'hôte, nommé. Témoin
navigateur : un tour avec `search_code`, le détail déplié contient au moins un chemin de résultat
et le compte.
