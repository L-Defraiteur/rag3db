# Vision — le fil de travail : relier le schéma, le code, les documents et la mémoire

4 octobre 2026. Idée de Lucie : « dans le cas d'une architecture de données,
l'agent peut très bien vouloir mémoriser que telle entité, telle relation, sert
à mettre en place tel sujet de code. Je fais de quoi parler d'user dans ma
base, et je veux me rappeler que c'est pour ma PR à propos des users, et que
c'est lié à tel design en mémoire, et sur disque à tel md. »

Ce document propose une forme. Rien n'est décidé ni codé. Il complète la
vision de la mémoire longue (`../3-octobre-2026-20h37/02`) et sa proposition
(`../3-octobre-2026-20h41/01`).

## 1. Ce que la demande ajoute

La mémoire longue telle qu'elle est proposée accroche une mémoire à **une
ligne d'une entité** (une fonction, une carte) ou à un **sujet**. La demande
de Lucie nomme trois choses que cette forme ne couvre pas :

1. **Une ancre sur le schéma lui-même** : l'entité `User`, la relation
   `FOLLOWS`, un champ — pas une ligne de la table, la table.
2. **Une ancre hors de la base** : un fichier `.md` sur disque, une PR, une
   branche, un commit — des choses qui ne sont pas indexées.
3. **Un point où tout cela se rejoint** : « ma PR à propos des users » relie
   une entité du schéma, du code, un design en mémoire et un document. Ce
   n'est pas une mémoire de plus : c'est ce qui tient les autres ensemble.

## 2. La forme : le fil de travail

Le troisième point a déjà sa pièce : le **sujet** (`Subject`), l'entité de
rendez-vous de la proposition. Un fil de travail est un sujet d'un genre
donné — une PR, un design, un chantier — auquel s'accrochent des ancres de
toute nature.

```
                 ┌─────────────── Subject « PR users » ───────────────┐
                 │                                                    │
   schéma : entité User      code : scope users.rs::create     mémoire : « on garde
   schéma : relation FOLLOWS  code : fichier migrations/007      l'email unique parce que… »
   disque : docs/users.md     dehors : PR #42, branche users
```

La question « pourquoi cette entité existe-t-elle ? » se lit depuis l'entité
vers ses fils ; la question « qu'est-ce que ma PR touche ? » se lit depuis le
fil vers ses ancres. C'est la même relation, dans les deux sens.

## 3. Trois genres d'ancre

Une seule relation d'ancrage, et une cible qui se dit de trois façons.

| Genre | La cible | Comment elle vieillit |
|---|---|---|
| **Ligne** | une ligne d'une entité indexée (scope, fichier, carte) | l'ingestion ou la synchronisation dit qu'elle a changé ou disparu — livré pour l'ingestion |
| **Schéma** | une entité, une relation ou un champ du catalogue | une migration ou un `register_entity` qui change la définition |
| **Référence** | un chemin sur disque, une adresse, un identifiant externe | une empreinte du contenu pour un chemin ; rien pour une adresse, seulement la date |

**L'ancre de schéma** demande une pièce qui manque : le schéma n'est pas dans
le graphe. La vision « le catalogue comme graphe »
(`vision_roadmap_09_2026/04`) le proposait déjà ; la forme la plus petite ici
est une entité `SchemaElement` (genre : entité, relation, champ ; nom ;
propriétaire) que le catalogue tient à jour à chaque enregistrement. Elle
devient une ligne comme une autre : `anchorable`, cherchable, et l'outil
`schema` peut rendre avec chaque entité les fils qui l'expliquent.

**L'ancre de référence** n'exige pas que la cible soit indexée. C'est une
ligne légère (`kind`, `ref`, empreinte, date vue). Un `.md` hors de l'index
s'y accroche par son chemin ; si le dossier est indexé plus tard, la
référence se résout vers la ligne et devient une ancre du premier genre.

## 4. Ce que l'agent fait

Pas un outil de plus pour chaque genre. Le verbe `remember` reçoit des ancres
de toute nature :

```
remember(
  claim: "L'entité User porte l'email unique : la PR users en dépend",
  subject: "PR users",
  anchors: ["schema:User", "schema:User.email", "path:docs/users.md", "pr:42"]
)
```

Et le rappel vient par ce que l'agent touche, par le crochet après outil :

- il appelle `schema` ou cherche dans `User` → « fils liés : PR users (3
  mémoires) » ;
- il lit `docs/users.md` → les affirmations accrochées à ce chemin ;
- il déclare ou modifie une entité → les mémoires accrochées passent « à
  revoir », et le rendu le dit.

Un fil se consulte comme un tout : `recall(subject: "PR users")` rend ses
ancres par genre, ses mémoires, et ce qui est à revoir.

## 5. Ce que le moteur donne et ce qui manque

| Besoin | État |
|---|---|
| Le sujet comme rendez-vous | proposé ; à rendre dérivé de ce qu'il porte |
| L'ancre vers une ligne quelconque (`anchorable`) | proposé (lot 2 de la mémoire) |
| « À revoir » quand une ligne change | livré pour l'ingestion |
| Le schéma comme lignes (`SchemaElement`) | à faire ; petit si le catalogue le pose à l'enregistrement |
| « À revoir » quand le schéma change | à faire : un événement à l'enregistrement, comme celui de l'ingestion |
| L'ancre de référence et son empreinte | à faire ; une entité légère |
| Le rappel par ce qu'on touche | crochet après outil : mécanique livrée, clients à écrire |

## 6. Pourquoi y penser maintenant

Parce que cela décide de la forme de l'ancre **avant** qu'elle soit codée. Si
l'ancre est écrite comme « une relation vers une ligne », le schéma et les
références devront être ajoutés par un second mécanisme. Si elle est écrite
comme « une cible d'un genre donné », les trois entrent par la même porte. Le
code de la mémoire longue n'a pas encore posé l'ancre : c'est le moment de la
dessiner large, et de n'en coder que le premier genre.

Générique : rien ici n'est propre au code. Un deck et ses règles, un dossier
et ses pièces, une personne et ses préférences sont des fils du même modèle.

## 7. Ce qui attend un choix de Lucie

1. Le mot : « fil de travail », « sujet », « dossier » — c'est le nom que
   l'agent lira dans ses outils.
2. Le schéma comme lignes du graphe : maintenant, pour que l'ancre de schéma
   existe dès la première version, ou plus tard.
3. Les références externes (PR, branche) : de simples étiquettes, ou une
   résolution vers git quand l'espace de travail est un dépôt.
