# Le plein texte de code-lourd compte plus de documents que de lignes

- **État** : ouvert (8 novembre 2026), pour la recherche et lucivy ; relevé par G.
- **Gravité** : dérive à expliquer (le plein texte de Scope rend plus de documents que la table n'a de
  lignes ; une recherche peut rendre un morceau qui n'existe plus, ou compter double).
- **Atteignable en service** : oui (code-lourd, l'hôte de luciepc, après un `index confirm` complet).
- **Touche rag3weaver** : oui (l'index plein texte lucivy de rag3weaver, son contrôle de fin d'index).
- **Famille** : produit

## Ce que c'est

Le 8 novembre 2026 à 12 h 09, un `index confirm` complet de code-lourd, par l'hôte de luciepc sur la lib
cbc06c3e1 (binaire 9208a2b6, master abf7020e0, lucivy-core 4.3.2), a fini en 4 min 30 s sur
« indexation terminée ». Son journal (`~/.cache/rag3weaver/commandes/index/1794136151873-454234.out` à
luciepc) porte :

```
plein texte : écart entre documents et lignes — Scope : 82958 documents pour 82855 lignes
```

103 documents de plus que de lignes dans le plein texte de l'entité Scope, juste après un index
complet. Les autres entités ne sont pas signalées. L'index n'a presque rien réembarqué : 137 741 vecteurs
sur 137 943 ont été réutilisés par leur contenu. Les lignes, elles, ont toutes été réécrites.

## Ce qu'on sait

- La vérification de fin d'index l'a vu et l'a dit, sans faire échouer l'index ; ce n'est pas un
  plantage.
- Le 25 octobre, la base de code de Lucie, reconstruite à neuf, rendait « autant de documents que de
  lignes (4 entités) ». L'écart n'est donc pas inévitable.
- Hypothèses à départager : des documents d'une passe précédente non retirés quand l'index réécrit
  toutes les lignes (les ids de scope changent, les anciens documents restent) ; ou des scopes
  supprimés entre le comptage des lignes et celui des documents (la montre tourne pendant l'index) ;
  ou une fusion lucivy en cours au moment du comptage.

## Ce qu'il faut

- Recompter à froid (`fts_counts` par daemon_status sur l'hôte de luciepc, ou sur une copie) pour savoir
  si l'écart dure ou s'il tenait au moment du comptage.
- S'il dure : retrouver les 103 documents sans ligne (leur id de scope) et dire d'où ils viennent ; puis
  un témoin rouge : un index complet sur une base qui a déjà des documents ne doit en laisser aucun
  sans ligne.
