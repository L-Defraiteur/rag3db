# La mission : une sonde de faisabilité

**Ce n'est pas une migration.** On ne fusionne rien, on ne rebase rien, on ne
touche pas au code de production. On répond à une question par oui ou par non,
et cette réponse décide d'un chantier de plusieurs semaines.

Lire d'abord [01 — Le repérage](01-reperage-ladybug.md) pour le constat, et
[03 — Notre greffe](03-notre-greffe.md) pour ce que fait notre code.

## 1. La question

> **La descente de prédicat de LadybugDB peut-elle atteindre un index qui ne
> répond pas à une comparaison mais à une recherche classée — un top-K avec
> score ?**

C'est ce que fait notre greffe pour la similarité vectorielle. C'est ce que leur
mécanisme fait pour l'ART, qui est ordonné.

- **Si leur descente est généralisée** : notre greffe du cœur se réduit, peut-être
  beaucoup, et on redevient un utilisateur avec des extensions.
- **Si elle est câblée pour les prédicats ordonnés** : nos lignes vivantes se
  réappliquent sur leur arbre, et le coût du suivi reste le renommage. Mais alors
  notre `IndexSearchBindData` est une **contribution amont évidente** — ils en
  auront besoin le jour où ils voudront pousser un prédicat vectoriel.

Les deux réponses sont utiles. Il n'y a pas de mauvaise issue, seulement une
question non répondue.

## 2. Ce que les signatures laissent déjà croire

À vérifier, pas à supposer — mais c'est le point de départ.

Les six virtuelles que Ladybug a ajoutées à `storage::Index` :

```
lookupAll            scanPrimaryKeyRange     discardPrimaryKey
lookupPrimaryKey     getStorageEntries       reclaimStorage
```

Toutes tournent autour de la **clé**. Aucune n'a la forme `(requête, k) →
(offset, score)`. Leurs commits disent la même chose : *« Add ART primary key
range scans »*, *« Guard ART range predicate pushdown »*, *« Support secondary
ART indexes »*.

**Hypothèse de travail** : leur généralisation et la nôtre sont orthogonales.
La sonde existe pour la confirmer ou l'infirmer, pas pour la présumer.

## 3. L'arbre de décision

S'arrêter dès qu'une branche tranche.

### Branche A — la lecture de leur optimiseur

`src/optimizer/filter_push_down_optimizer.cpp` chez eux a bougé de 170 lignes.
Y chercher **ce qui décide qu'un prédicat descend**. Trois formes possibles :

1. **Un test sur la forme du prédicat** (`=`, `<`, `BETWEEN`) → un prédicat de
   similarité n'entrera jamais. Réponse : orthogonal.
2. **Un test sur le type d'index** (`IndexType == ART`) → il faudrait élargir le
   test. Estimer la taille : sous une centaine de lignes, c'est une contribution.
3. **Un crochet générique** que l'index déclare → réponse : généralisé, on
   s'y branche.

**Cette lecture seule peut suffire à répondre.** Ne pas compiler avant d'en
avoir besoin.

### Branche B — si la descente est atteignable

Alors écrire la coquille minimale : un index qui déclare savoir répondre à une
recherche classée, et vérifier qu'un `WHERE vector_search(...)` descend jusqu'à
lui. Ça se fait dans un `git worktree` séparé, jamais dans l'arbre de travail.

### Branche C — si elle ne l'est pas

Mesurer alors le coût de la **réapplication** de nos lignes sur leur arbre. Le
[document 01](01-reperage-ladybug.md) donne la liste : 22 de nos 24 fichiers
partagés ont bougé chez eux, dont nos deux sites de greffe principaux
(`node_table.cpp`, 326 lignes d'écart ; `filter_push_down_optimizer.cpp`, 170).

Les relire un par un et répondre : **combien de nos greffons se reposent tels
quels, combien demandent une réécriture ?**

## 4. Le critère de réussite

La sonde est finie quand ces trois phrases peuvent être écrites **et étayées par
un fichier et une ligne de leur code** :

1. « Leur descente de prédicat *atteint* / *n'atteint pas* un index qui répond par
   un top-K classé, parce que ⟨…⟩. »
2. « Donc notre greffe *se dissout* / *se réapplique* / *devient une contribution
   amont*, et son coût est de ⟨estimation⟩. »
3. « Le rebasage complet coûterait ⟨estimation⟩, dont ⟨…⟩ pour le renommage et
   ⟨…⟩ pour les 187 fichiers partagés hors du cœur. »

**Ce n'est pas une réussite de produire un patch.** C'est une réussite de
produire ces trois phrases.

## 5. Un nettoyage à faire au passage, indépendant de la réponse

`src/include/processor/operator/scan/fts_scan_node_table.h` n'est inclus par
personne. `src/include/common/fts_types.h` n'est inclus que par lui. Environ
**64 lignes de code mort**, résidus du retrait de l'extension C++ `lucivy_fts`
(commit `a39698fd4`).

À supprimer, avec un test de compilation derrière. Ça n'attend pas la sonde, et
ça réduit d'autant la surface à reporter.

## 6. Ce qui ferait abandonner en cours de route

À dire tout de suite plutôt que de s'entêter :

- **Le build de Ladybug ne passe pas sur ce poste.** Répondre par la lecture du
  code seule, et le dire — une réponse lue vaut mieux qu'une absence de réponse,
  tant qu'on ne la présente pas comme une réponse mesurée.
- **Leur optimiseur est illisible sans le reste de leur refonte.** Alors la
  réponse est « non évaluable à ce coût », ce qui est une réponse.

## 7. Ce qu'il ne faut surtout pas faire

- **Ne pas fusionner, ne pas rebaser, ne pas renommer.** La sonde est en lecture.
  S'il faut compiler, `git worktree` séparé, et le dire.
- **Ne pas toucher à `extension/rag3weaver/`.** C'est un autre chantier, avec
  d'autres sessions dessus, et sans rapport avec celui-ci.
- **Ne pas ajuster un compteur de test sans comprendre ce qu'il compte** — un
  compteur qu'on ajuste sans savoir est un test qu'on a désactivé.
- **Ne pas conclure sur la foi d'une signature.** C'est l'erreur que ce document
  a déjà corrigée une fois : la première version de la mission portait sur le
  plein texte, jusqu'à ce qu'on regarde qui appelle réellement le code.

## 8. Deux pistes pour plus tard, pas pour maintenant

- **`Vela-Engineering/kuzu`** : un second fork MIT, orienté mémoire d'agent avec
  **écriture concurrente multi-processus**. Ça touche un mur qu'on a heurté (deux
  processus sur une même base, `F_WRLCK`). Non évalué. Si la sonde tranche vite,
  c'est le repérage suivant.
- **Le renommage `kuzu` → `rag3db`** coûte 1 608 fichiers à chaque reprise de
  l'amont. Il mérite qu'on demande ce qu'il rapporte. Pas la question de cette
  mission, mais la sonde en donnera l'occasion.
