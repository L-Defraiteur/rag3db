# Des catégories nommées qui sont elles-mêmes des fiches

17 novembre 2026. Vision de Lucie, à conserver pour la mémoire de rag3weaver.
Elle décrit une direction de conception, pas une capacité déjà livrée ni une
demande de commencer son implémentation.

## L'idée

**Une déclaration de catégorie est elle-même une fiche de mémoire recherchable.**
Elle porte un nom, une description et les valeurs qu'elle définit. Les agents
peuvent construire ces vocabulaires partagés et les retrouver, comme les autres
fiches, au lieu de réinventer des chaînes arbitraires dans chaque document.

Les autres fiches référencent la déclaration ou une valeur précise de celle-ci.
Ces références sont des relations exploitables : elles permettent de filtrer
les fiches sur une catégorie entière ou sur l'une de ses valeurs.

Le terme « enum nommé » employé dans la conversation désigne ce vocabulaire
déclaré dans les données. Ses définitions ne sont pas nécessairement figées
dans une énumération Rust : les agents doivent pouvoir les créer et les faire
évoluer par les mécanismes autorisés de la mémoire. Les fiches de définition
peuvent aussi être organisées ensemble en vocabulaires communs.

## Trois opérations distinctes

| Intention | Résultat attendu |
| --- | --- |
| Chercher une catégorie par son nom ou sa description | Retrouver sa fiche de définition et les valeurs disponibles. |
| Filtrer les fiches qui référencent une catégorie | Retrouver toutes les fiches concernées, quelle que soit la valeur référencée. |
| Filtrer les fiches qui référencent une valeur de cette catégorie | Retrouver seulement celles qui emploient cette valeur précise. |

Par exemple, la fiche de définition **Rôle narratif** déclare les valeurs
**Protagoniste**, **Antagoniste** et **Témoin**, avec leurs descriptions.
Trois fiches de personnages utilisent respectivement Protagoniste, Témoin et Témoin.
Le filtre sur Rôle narratif retrouve les trois ; le filtre sur sa valeur Témoin
ne retrouve que les deux dernières. La définition reste elle-même trouvable
en cherchant le vocabulaire des rôles d'un récit.

L'exemple narratif n'enferme pas la capacité dans la fiction. Le même mécanisme
peut décrire des types de documents, des rôles d'agents, des états de workflow
ou des statuts de connaissance.

## Des liens entre fiches sans lien direct préalable

Une fiche peut déjà pointer vers une source, du code ou une autre fiche.
La catégorie partagée apporte un autre chemin de découverte :

```text
Fiche A ──référence──> Valeur V <──référence── Fiche B
                           │
                    appartient à
                           │
                  Fiche de catégorie C
```

Les deux fiches n'ont pas besoin de se connaître pour être retrouvées ensemble.
On veut pouvoir demander « quelles fiches utilisent cette catégorie ? »,
« lesquelles utilisent cette valeur ? », ou combiner plusieurs de ces filtres
avec une recherche dans la mémoire.

Ces relations peuvent se parcourir par la définition et ses références ; il
n'est pas nécessaire de matérialiser une arête entre toutes les paires de fiches
qui partagent une valeur. Le filtre doit porter sur les références structurées,
pas seulement sur la présence du même mot dans leur texte.

Partager une valeur constitue une piste de rapprochement, pas une preuve que
deux fiches parlent du même objet. Partager une catégorie générale est encore
moins précis : cela ne doit ni fusionner les fiches ni rendre tous leurs sujets
équivalents. Un éventuel signal de classement structurel reste à qualifier.

## La cohérence portée par les références

Pour la conception future, conserver les distinctions suivantes :

- La référence à une catégorie et la référence à une de ses valeurs sont
  différentes. On doit vérifier que la valeur appartient à la catégorie désignée.
- L'identité d'une définition ou d'une valeur doit pouvoir rester stable quand
  son nom affiché change. Deux valeurs homonymes de catégories différentes ne
  sont pas automatiquement la même valeur.
- Une valeur inconnue ou une référence ambiguë se signale ; elle ne devient
  pas silencieusement une nouvelle catégorie. La proximité sémantique peut
  aider à retrouver une définition, sans remplacer le choix explicite.
- Une évolution de sens doit permettre de retrouver les fiches dépendantes et
  de réexaminer leur validité. Elle n'autorise pas à réécrire automatiquement
  leurs affirmations passées.
- Les références et les filtres respectent la vue et les accès de l'appelant.
  Une définition hors de sa vue ne doit pas être remplacée par un homonyme global.
- Employer une valeur valide ne prouve pas qu'une affirmation est vraie :
  classification, provenance et degré de certitude restent distincts.

La représentation exacte des identifiants, les cardinalités des références et
les règles de révision sont à concevoir. Aucun schéma concret n'est imposé ici.

## Composer avec la mémoire existante

La première piste est un gabarit de fiches de définition, des relations vers
les catégories et leurs valeurs, puis les filtres et parcours correspondants.
Il faut d'abord examiner ce que les entités, relations, gabarits et outils de
mémoire expriment déjà, avant d'ajouter un mécanisme au runtime.

L'interprétation libre de nouveaux formats ou d'attributs par un modèle,
évoquée plus loin dans la conversation, est une autre piste. Elle n'est pas
nécessaire pour déclarer des catégories recherchables et les référencer.

## Provenance de cette vision

Conversation fournie par Lucie dans **Recommandations horreur lovecraftienne.md** :
passage sur les enums nommés vers les lignes1060–1175, découverte des fiches par
valeurs partagées vers1254–1360, précision sur une réalisation par gabarits
vers1918–1978. Seuls ces éléments de conception sont repris ici.

Clarification de Lucie dans la session du17novembre : les déclarations sont
elles-mêmes des fiches cherchables ; les autres fiches peuvent les référencer,
et on veut filtrer aussi bien par catégorie que par valeur. Cette formulation
fixe le centre de la vision.
