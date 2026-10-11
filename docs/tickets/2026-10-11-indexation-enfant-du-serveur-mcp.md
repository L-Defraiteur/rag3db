# L'indexation est un enfant du serveur MCP et meurt avec lui

- **État** : partiel (11 octobre 2026). La fermeture propre et SIGTERM sont réglés : la marche 0 de la
  forme C (`75de6618f`) arrête l'indexation au prochain paquet validé et l'attend, et la mémoire
  longue traite SIGTERM comme la fermeture du tube (`59718e806`) ; relancer `index` reprend là. Reste
  le serveur tué (SIGKILL, plantage), que règle le démon de la forme C
  (`extension/rag3weaver/docs/11-octobre-2026-arbre-principal/01-le-demon-possede-le-catalogue.md`).
- **Gravité** : travail long perdu sans rien dire — et la base reste à moitié indexée
- **Atteignable en service** : oui, **dès le premier usage** : c'est le premier geste d'une nouvelle installation
- **Touche rag3weaver** : oui — l'outil `index` du gabarit `code`, et tout travail long lancé en fond par un outil

## Ce que c'est

L'outil `index` **rend aussitôt** et lance l'indexation en fond :

```
indexation lancée en fond.
journal : ~/.cache/rag3weaver/commandes/index/1791674068001-1317084.out
7172 fichiers retenus (60.7 Mo) : 4135 code, 3037 texte
```

Mais ce travail en fond est un **enfant du processus serveur**. Quand le client
MCP ferme son tube — ou simplement quand la session Claude Code se termine — le
serveur sort, et l'indexation meurt avec lui. **Sans un mot** : ni le journal ni
la réponse de l'outil ne disent que le travail a été interrompu.

Mesuré deux fois le 11 octobre 2026 en jouant la recette, sur
`~/git_workspaces/rag3db-lourd` (7 172 fichiers, 60,7 Mo) :

| essai | fenêtre du tube | résultat |
|---|---|---|
| 1 | fermé juste après `index` | 79 s, base vide (4 096 octets) |
| 2 | 10 minutes | `usages` répond encore « Catalogue occupé », puis mort |

**Un vrai client MCP garde son tube ouvert**, donc le cas ne se présente pas à
chaque appel. Mais il se présente **à chaque fin de session**, et une session
Claude Code se ferme souvent avant sept minutes.

## La forme générique attendue

**Le travail long appartient au démon**, pas au serveur MCP. C'est la décision de
Lucie — « installer rag3weaver, c'est installer un démon » — et c'est déjà la
forme de la commande en fond livrée par l'arbre principal (`run` en fond + la
lecture du journal) :

1. le serveur MCP **confie** l'indexation au démon et rend la **poignée** et le
   **chemin du journal** ;
2. le travail survit à la session : une autre session, ou la même relancée,
   retrouve la poignée et lit le journal ;
3. **une reprise entre deux paquets finit proprement** — l'indexation est déjà
   découpée en paquets, donc une interruption laisse un point de reprise plutôt
   qu'une base à moitié écrite.

Et **sans démon**, la réponse de `index` doit le **dire** :

> indexation lancée en fond — **elle s'arrêtera avec ce serveur** : gardez cette
> session ouverte, ou lancez un démon (`--demon <adresse>`) pour qu'elle lui
> survive. journal : …

C'est la même règle que partout ici : un repli silencieux est une information
que personne ne consulte. Aujourd'hui la réponse annonce un travail en fond sans
dire de quoi il dépend pour vivre.

## Le témoin

Aucun, et l'absence est assumée : éprouver ça demande une indexation réelle (sept
minutes au moins) et la fermeture d'un tube au milieu. Le témoin honnête n'est
pas un e2e mais la recette elle-même (`templates/mcp/README.md`), qui porte
désormais l'avertissement. **Quand la forme générique sera écrite**, le témoin
devient facile et vaut la peine : lancer `index` par un serveur relié à un
démon, tuer le serveur, vérifier que le journal continue d'avancer.

## Ce qu'il faut pour le fermer

1. la réponse de `index` dit de quoi dépend le travail en fond (une ligne, sans
   démon) — **ça peut partir tout seul, et ça évite la surprise** ;
2. la forme générique : le travail confié au démon, la poignée rendue, la reprise
   entre paquets ;
3. son témoin, qui n'a de sens qu'après (2).

## Ce que ça ne bloque pas

La recette « une base par session » marche, **à condition de garder la session
ouverte pendant le premier index** — c'est écrit dans `templates/mcp/README.md`.
Les index suivants sont incrémentaux et courts.
