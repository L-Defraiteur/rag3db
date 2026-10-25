# Le juge des outils — deux rapports, et des défauts typés

Demandé par Lucie le 25 octobre 2026 au soir, relayé et tranché par c7. Le lot suit le juge de
session (page 04, sur master en 4b64a98e5).

Lucie : « faudrait plus détaillé les jugements sur outils, genre "missing additional parameter"
et "parameter has bad format/must be optional" etc. qu'il ait un tool judge tools et après un tool
judge session et les 2 c'est ses rapports finaux. […] qu'il hésite pas à faire propositions pour
changer format en gros. » Et sur le retour : « si je te fais le résumé d'un livre mais je te sors
aussi toute la famille de son auteur et son enfance au Brésil alors que pas dans le livre ».

## Deux rapports, à la fin de l'enquête

L'enquête ne change pas. À sa fin, le juge rend deux verdicts, chacun dans sa forme fixe.

- **`bilan_des_outils`** : un objet par outil appelé, avec :
  - ses appels, ses échecs et ses appels inutiles ;
  - **la part du retour que l'agent a utilisée** (une estimation, 0 à 1) ;
  - ses **défauts**, rangés en trois listes, `bloquant`, `gênant` et `mineur`, toujours
    présentes, chacune dans l'ordre des appels.

  Ce rapport juge l'outil, pas l'agent.
- **`bilan_de_session`** : ce qu'on a déjà (résumé, note, confiance, objectif, faits dits ou
  déduits, frictions, attentes, comparaison aux bilans précédents de la même tâche). Il **renvoie
  aux défauts bloquants** des outils qu'il a appelés, par outil et genre, sans les redire.

Les deux sont demandés l'un après l'autre, sur la même conversation. Le premier est donné au
second.

## Les défauts, typés

Chaque défaut porte quatre éléments :
- son `genre` ;
- sa `preuve` : le `call_id`, le texte exact (les arguments envoyés, ou l'extrait du retour reçu)
  et, pour un retour, ce que l'agent en a fait au tour suivant ;
- sa `gravite` (la liste où il est rangé) ;
- sa forme `proposee`, entière : un fragment de schéma de paramètres, la forme du retour, ou la
  description.

Un défaut sans preuve ne vaut rien. Les genres :

| genre | ce que c'est |
|---|---|
| `parametre_manquant` | l'agent avait besoin d'un paramètre qui n'existe pas (il a contourné par plusieurs appels, ou abandonné) |
| `parametre_mal_nomme` | le nom envoyé n'est pas au schéma, ou le nom déclaré fait croire autre chose |
| `parametre_devrait_etre_optionnel` / `parametre_devrait_etre_obligatoire` | |
| `format_de_parametre` | type ou forme mal devinée (liste ou chaîne, chemin relatif ou absolu, entier ou texte) |
| `retour_mal_forme` | le retour ne se lit pas comme sa forme l'annonce (json cassé, colonnes décalées, deux formes mêlées) |
| `retour_incoherent` | le même outil rend des formes différentes d'un appel à l'autre, ou contredit un outil voisin pour la même chose |
| `retour_bavard` | la bonne information, noyée (répétitions, en-têtes redits) ; preuve : la part utilisée contre la taille |
| `retour_hors_sujet` | de l'information non demandée, que l'agent doit trier : autre chose que ce qu'il fallait, pas trop de la bonne chose |
| `retour_incomplet_ou_ambigu` | le champ qu'il fallait manque (total, provenance, un « rien trouvé » qui ne dit pas où il a cherché), ou le texte se lit de deux façons (avec `retour` et `cru`) |
| `description_trompeuse` | la description fait croire autre chose que ce que l'outil fait |
| `erreur_muette` | un échec qui ne dit pas quoi faire à la place |
| `outil_manquant` | l'outil voulu n'existe pas |
| `outils_a_fondre` | deux outils que l'agent confond, ou enchaîne toujours |
| `autre` | ce qu'aucun genre ne couvre, avec un `libelle` court et libre |

Un libellé `autre` qui revient sur plusieurs runs est le candidat au prochain genre. La page par
outil groupe ces défauts par libellé.

## Ce que le juge voit

`lire_le_run` rend en plus, pour chaque outil appelé, **sa déclaration telle que l'agent l'a
vue** : son schéma de paramètres et sa description, pris dans les `tool_defs` du backend (ce que
l'agent a reçu, pas le manifeste brut). Sans elle, le juge ne pourrait que deviner un nom trompeur
ou un paramètre qui devrait être optionnel.

Le message système dit quatre choses :
- proposer un changement de forme (paramètres, retour, découpage des outils) est voulu et
  attendu, pas seulement une description reformulée ;
- un défaut sans preuve ne vaut rien ;
- le retour se juge tel que l'agent l'a reçu, pas tel que l'outil le documente ;
- un retour que l'agent a dû relire ou redemander est un défaut de l'outil avant d'être une
  friction de l'agent.

## Le rangement (c7, relu par 96)

- **Une fiche par outil, qui suit la définition, pas l'instance** (96, c7) :
  - un outil dont le graphe est livré (`@builtin:` ou `templates/tools/`) a une fiche commune,
    « outil : <nom> », quels que soient les backends qui l'exposent ;
  - un outil dont le graphe vient du manifeste d'un backend a « outil : <backend> / <nom> ».

  La fiche est marquée `via: juge` et ancrée deux fois : `outil:<nom>` (tous les homonymes) et
  `gabarit:<chemin du graphe>` (le code jugé).
- Chaque run jugé y ajoute un L1 de section exactement `defauts`, de source le journal du run. Son
  texte donne les défauts de cet outil dans ce run, classés par gravité, avec la part utilisée, **le
  backend du run** et le bloc json.
- **La provenance de chaque apport** passe par `citer_source` : `doc:sources/<jour>-<run>.md#bilan`
  ou `#defauts`. Depuis le journal d'un run, `cited_by` rend la fiche de la tâche et toutes les fiches
  d'outils qu'il a nourries. Une même source porte ainsi plusieurs apports, sur plusieurs fiches :
  la clé (fiche, source, section) reste unique (96).
- **Le bilan de session** reste sur la fiche de la tâche (page 04), avec son renvoi aux défauts
  bloquants.
- `cited_by(value: "outil:read_file")` rend la fiche de l'outil et les fiches des tâches qui l'ont
  appelé.

## Ce qui devient une proposition (c7)

- **`description_trompeuse` seul** devient une proposition, sur la clé `description` de l'outil
  dans le manifeste jugé, comme aujourd'hui.
- **Tous les autres genres** sont des défauts rangés dans la fiche de l'outil, avec leur forme
  proposée entière, jamais appliqués. Le schéma des paramètres vit dans le `.mmd` (`%% param:`),
  souvent un gabarit livré partagé, hors du dossier du backend.
- Un ticket pour un défaut récurrent d'un outil livré suit la règle de Lucie (un défaut connu
  devient un ticket). C'est un pas de plus, à part, au mot de c7 après ce lot.

## La vue

`/ide/bilans` gagne une page par outil, qui est sa fiche :
- ses défauts par gravité, puis dans l'ordre ;
- la tendance par genre d'un run à l'autre ;
- les `autre` groupés par libellé.

Les fiches de tâche renvoient à leurs outils.

## Les témoins

- **Sur faux modèle (e2e_juge)** : les deux rapports et leurs deux formes ; la fiche par outil et
  son L1 « defauts » classé par gravité ; le renvoi du bilan de session aux défauts bloquants ;
  `lire_le_run` avec les déclarations ; un `autre` à libellé ; la vue par outil.
- **Sur Gemini, jugé sur le résultat** : un outil de banc au schéma volontairement faux (un
  paramètre obligatoire qui devrait être optionnel, et un nom trompeur), puis un outil dont le
  retour mêle du hors-sujet et change de forme entre deux appels. Le juge nomme les quatre défauts
  (`parametre_devrait_etre_optionnel`, `parametre_mal_nomme`, `retour_hors_sujet`,
  `retour_incoherent`), chacun avec son `call_id`, et propose la forme corrigée.

## La suite (visions/00-vision-generale.md §7)

La graine de Lucie, « Le juge sur les outils que les agents déclarent : bac à sable et
surveillance », dit l'ordre :
1. le bilan des outils (ce lot) ;
2. la marque `a_surveiller` ;
3. l'estampille de l'outil déclaré : auteur, run, instant (la proposition les a, le manifeste
   appliqué les perd) ;
4. la réaction qui adresse ;
5. le bac à sable, avec ses essais.

Rien de cela n'entre dans ce lot.
