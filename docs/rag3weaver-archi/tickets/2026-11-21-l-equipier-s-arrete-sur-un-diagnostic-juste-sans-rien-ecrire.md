# L'équipier s'arrête sur un diagnostic juste sans rien écrire

- **État** : ouvert, le 21 novembre 2026 ; pour archi
- **Famille** : produit
- **Pour** : archi
- **Gravité** : moyenne. Un correctif d'une fonction, diagnostiqué juste, n'est pas fait.
- **Atteignable en service** : oui
- **Touche rag3weaver** : oui, la règle d'arrêt « 3 pas de suite sans élément réussi » (`daemon_host/teammate.rs`)
- **Signalé par** : archi, rejeu de nuit B, le 21 novembre 2026

## Ce qui s'est passé

Le rejeu B (le bac à sable sans sccache, depuis 81ccf05db4) a donné un diff **vide** : 509 s, 3 pas,
78 appels au modèle. Au deuxième pas, la boîte porte le bon diagnostic :
`variables_du_poste_hors_du_bac` laissée vide dans `commande.rs`, et le témoin nommé. L'équipier est
ensuite parti dans `scripts/poste` et dans `~/.cache/rag3weaver-build/poste`, que le ticket cite dans
son récit. Il a joué `test_poste.sh` et `cargo config get`, sans jamais écrire. L'hôte l'a arrêté sur
« 3 pas de suite sans élément réussi ». Dossier :
`~/.cache/rag3weaver-build/campagne/B-le-bac-a-sable-sans-sccache/20261120-235851` (tablette).

## Le pas exact qui bloque

1. Le plan n'a pas d'élément « corriger la fonction diagnostiquée ». Le diagnostic reste dans un
   rapport et n'entre pas dans le plan.
2. L'arrêt compte trois pas sans élément réussi, sans regarder qu'un diagnostic venait d'être posé.
3. Accessoire : les lectures hors de l'arbre (`~/.cache/rag3weaver-build/poste`) ont été accordées
   par le superviseur automatique du lanceur. En service, un humain les aurait vues.

## Ce qui est attendu

Un diagnostic qui nomme un symbole et un témoin devient un élément du plan (corriger, puis jouer le
témoin). La porte de vérification du ticket voisin
(`2026-11-21-l-equipier-finit-un-correctif-sans-porte-de-verification.md`) donne au pas suivant le rouge du
témoin comme suite, au lieu d'une exploration.

## Comment le prouver

Par le faux modèle : un pas qui rapporte un diagnostic avec symbole et témoin ajoute au plan un élément
« corriger » ; trois pas sans élément réussi après un tel diagnostic ne s'arrêtent pas avant que la porte
ait joué le témoin.
