# Doc 08 — TODO : migrations destructives (à designer plus tard)

Date : 12 mars 2026

## Contexte

Doc 07 couvre les migrations **additives** (ajout de champs, nouveaux signals, etc.). Les cas destructifs sont refusés avec erreur pour l'instant. Ce doc liste ce qu'il faudra designer.

Note : la plupart de ces cas sont destructifs dans **n'importe quelle DB** (Postgres, MySQL, etc.) — pas spécifique à rag3db. Un DROP column perd les données partout, y'a pas de UNDO DROP. La question c'est juste comment on l'expose à l'utilisateur.

## Cas à traiter

### 1. Suppression de champ

- `ALTER TABLE DROP {col}` supporté par rag3db
- Destructif dans toute DB — pas de revert possible
- Si le champ était content/title → needs_reindex aussi
- Piste : supporter directement, log warning "données perdues pour {field}"

### 2. Changement de type d'un champ

- Pas de `ALTER COLUMN TYPE` dans Kuzu (à vérifier)
- Chemin possible : add new col → CAST + migrate data → drop old col
- Piste : refuser, proposer drop + re-add manuellement (même résultat, explicite)

### 3. Changement d'embedding dim

- Nécessite recréer les chunk tables (colonnes FLOAT[N])
- Re-embed tout (changement de modèle = reconstruction complète de toute façon)
- Piste : `catalog.rebuild()` complet
