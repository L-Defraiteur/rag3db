# 01 — Où on va, et où on en est

*29 août 2026, midi. Rappel de la vision, des ambitions, et de l'état réel.*

## 1. La thèse, en une phrase

Un agent dont **le substrat est aussi le sujet** : il vit dans une base de
données, il la gère, il construit avec la technologie dont il est fait, et ce
qu'il construit retourne dans la base où il vit
([doc 01](../vision_roadmap_09_2026/01-la-vision.md)).

Une seule idée la porte : **tout est un graphe, et un graphe est une donnée**.
Un pipeline d'ingestion est un graphe. Un outil est un graphe plus une fiche.
Un agent est un sous-graphe compilé. Un backend est un ensemble de graphes plus
une surface. Le catalogue de tout ça, ce sont des entités dans la base.

D'où la conséquence qui n'est pas décorative : **un agent cherche ses propres
capacités avec les mêmes moyens qu'il cherche un document**. Il n'a pas une
liste d'outils, il a un RAG sur ses outils.

## 2. La cible immédiate : des catalogues de gabarits

Décidée le 29 août ([doc 08](../vision_roadmap_09_2026/08-des-catalogues-de-gabarits.md)).
Ne pas leur faire tout coder : on tient d'avance des `user`, des
`conversation`, des `product` — un agent les **pose**, puis les modifie.

**Quatre familles**, dont une transversale :

| famille | ce que c'est | état |
|---|---|---|
| entité | un `EntityConfig` : champs, signaux, découpage | **3 fournis** |
| graphe | un outil (`.mmd`) | **6, ils existaient déjà** |
| composant | React, côté front | à faire |
| **motif** | s'applique à une entité au lieu de la remplacer | **1 fourni** (`versioned`) |

**Deux axes pour ranger**, et ils répondent à deux questions : la **famille**
est structurelle et fermée (*ce qu'on peut en faire*), la **catégorie** est
thématique et ouverte (*de quoi ça parle*). Un `user` est de famille `entity`
et de catégorie `auth` ; un écran de connexion sera de famille `component` et
de la même catégorie. C'est ce qui permettra de demander « tout ce qui touche à
l'authentification » et d'obtenir le schéma **et** l'écran.

**Quatre verbes** : chercher, poser, adopter sous un nom, modifier. Le
quatrième est le plus important — *rien ne doit se casser si on change un
gabarit posé*.

## 3. Ce qui marche aujourd'hui

- **Le catalogue s'indexe** : 10 gabarits lus du disque, 10 indexés, 0 échec.
- **Les facettes sont exactes** : `category=auth` rend exactement `user` ;
  `family=entity` rend exactement les trois.
- **Poser marche**, sous le nom qu'on veut.
- **Le motif `versioned` marche** : `hashsafe [sku]` devient `[sku, revision]`,
  deux révisions font deux lignes, l'entité nue et la versionnée n'ont pas la
  même identité. Aucun mécanisme nouveau — `hashsafe` *est* la politique
  d'identité, le motif nomme un choix qui existait.
- **L'embedder est juste** : 3/3 sur les trois questions, mesuré au cosinus nu.

## 4. Ce qui ne marche pas, et qui bloque la thèse

Deux défauts, ouverts en issues le même jour :

- [**Le chemin vectoriel du moteur ne classe pas**](../issues/29-08-2026/01-le-vecteur-du-moteur-ne-classe-pas.md).
  Le cosinus nu est juste 3/3, le même calcul par `Catalog::search` est faux
  2/3, et `user` sort premier aux trois questions. Ni l'embedder, ni le
  cross-encoder, ni le filtre, ni le catalogue.
- [**Le plein texte ne trouve rien sur une phrase française**](../issues/29-08-2026/02-le-plein-texte-ne-trouve-rien-en-francais.md).
  La recherche floue colle la requête en un seul mot. Le moteur l'avertit dans
  `meta.warnings`, que personne ne lit.

Tant que le premier tient, un agent trouvera un gabarit s'il connaît sa
catégorie, pas s'il décrit son besoin. C'est exactement la moitié qui compte.

## 5. Les autres directions, et pourquoi elles attendent

- **La parole (TTS/STT)** — le bon différenciateur produit : un développeur *a
  déjà* un LLM, il *n'a pas* de moteur de parole. Bloquée en partie par des
  bugs burn, dont 16 rapports prêts et jamais envoyés.
- **Le terminal à plusieurs** — bus, diffusion, postures, interruption, traces
  et artefacts existent. Manquent le terminal, l'approbation humaine, et le bac
  à sable dont `Cwd` et `RootPolicy` sont maintenant posés.
- **Le schéma comme programme** — des backends écrits par des agents.
  `lifecycle`, `SchemaCost`, `declared()` y vont.
- **MCP** — la feuille de route le range en « puis seulement ».

## 6. La leçon de méthode de la semaine

**Sept mécanismes construits, documentés, jamais atteints depuis le chemin
réel** : `BurnDevice::for_role`, `Postures::describe_for`, la borne des lots
d'embedding, `pause_dialogue`, `FlushConfig::embed_batch_size`, le graphe
hybride de `search`, `Device::vulkan`.

Ce qui n'a pas de test **de bout en bout par le chemin de l'utilisateur**
n'existe pas, même avec des tests unitaires verts. Avant d'ouvrir une
direction : une heure à chercher ce qui est déjà là et débranché.

Corollaire du 29 août : **un test qui passe ne prouve rien s'il n'assère que
l'appartenance**. Le premier test du catalogue vérifiait qu'une liste de dix
contenait `user` — vrai quel que soit l'ordre. Il fallait affirmer le
**premier**, et c'est là que les deux bugs sont apparus.
