# La migration de la fiche de Lucie — 25 octobre 2026, 23 h

La mémoire longue de Lucie passe du gabarit memory du 19 octobre (Memory, Subject, Ref,
RefType) au gabarit livré sur master ce soir (neuf entités, les fiches en fichiers, le format
versionné, le manifeste du dossier). Mot de c7 à 23 h 00, après le « relancé » de G (Rust
159087582, lib 04063ee06). Rien n'est effacé ; ce que l'ancien modèle tenait est recopié dans
une fiche, et les deux lignes d'origine restent jusqu'à ce que l'archiviste (M4) sache les
relire.

## Le déroulé, par l'hôte, sans l'arrêter

L'hôte sert aussi la base de code ; tout passe par ses routes, le jeton local en Bearer et une
session (`POST /session`). Les scripts sont dans le scratchpad de la session (`migration-a.sh`,
`migration-b.py`) ; ils n'affichent jamais le jeton.

1. **Prévenir** c7 et G une minute avant.
2. **Fermer** le seul backend memoire-longue : `POST /backend/close {"manifest": …}` →
   `closed: true`. Un backend fermé reste connu de l'hôte, et **tout appel le rouvre** : il
   fallait enchaîner tout de suite.
3. **L'instantané** : `instantanes.sh migration-fiche-25-oct ~/.local/share/rag3weaver/memoire-longue.rag3db`
   (le script de l'arbre principal ; il refuse si un processus tient un fichier). Cinq fichiers,
   la base à 14 032 896 octets, `RETOUR.sh` à côté, sous
   `~/.local/share/rag3weaver/instantanes/migration-fiche-25-oct/`.
4. **Le manifeste au gabarit livré** : l'ancien dossier `~/.config/rag3weaver/memoire-longue/`
   mis de côté en `memoire-longue.avant-25-oct/` ; une copie fraîche de `templates/` à master
   à sa place, avec trois champs en absolu : `database`
   (`~/.local/share/rag3weaver/memoire-longue.rag3db`), `files_root` (`/home/lucied/memoire-longue`,
   choisi par c7 pour Lucie, qui tranchera au matin : déplacer le dossier avec son manifeste
   suffit), `vector_extension` inchangé.
5. **Rouvrir** : `POST /backend/open {"manifest": …}` → ok, vingt outils, la réaction
   `review_anchored`. L'ouverture écrit `manifeste.yaml` et compare le dossier à la base.
6. **Écrire**, par `POST /call {"manifest", "name", "arguments"}` : `create_ref_type doc`
   (confirmé ; sa mémoire n'avait que le genre « fichier » du 19 octobre), `put_source`,
   `put_fiche`, `put_apport`, `relier_apport`, `citer_source`.
7. **Relire** : `get_fiche`, `recall_fiches`, `recall_apports`, `cited_by`, et les deux lignes
   d'origine.

## Avant et après : les deux lignes d'origine, identiques champ à champ

| champ | Memory 1fb42f63-fe94-15c4-65da-646c80d37dbb (23 h 03 et 23 h 05) |
|---|---|
| claim | Un duty cycle à 70 % garde l'écran utilisable pendant une passe e2e. |
| why | Mesuré : à 100 % le compositeur figeait. |
| said_as | régime GPU |
| kind, origin, reach | fact, inferred, person |
| state, revision | current, 1 |
| created_at, updated_at | 1791682063181, 1791682063181 |
| reviewed_at | null |

Le Subject « régime GPU » : about « Comment on ménage les cartes du poste. », state current,
avant comme après.

## Ce qui existe en plus, dans `/home/lucied/memoire-longue/`

- `manifeste.yaml` : gabarit memory v1, format 1, les trois entités à fichiers (Fiche composée,
  RefType et Source propres), la liste des genres, les relations et leur sens.
- `sources/2026-10-25-migration.md` : la Source 44275045, origine `migration`, via
  `migration`, une section « régime GPU » qui porte la claim, le pourquoi et la provenance
  (l'uuid, les dates, le Subject).
- `fiches/e1ebfc2d-45e3-77e6-f622-7ebbcd938059.md` : la Fiche « régime GPU » (about « Comment
  on ménage les cartes du poste. », current, via `migration`, revision 1, created_at
  1792965844484), son Apport L1 ba482079 (section « régime GPU », texte = claim et pourquoi,
  current) relié par `A_POUR_APPORT`, qui cite par `APPORT_CITE` la Ref `doc`
  « sources/2026-10-25-migration.md#régime GPU ». Le bloc machine porte les trois lignes et
  leurs deux liens.
- `genres/doc.md` : le genre `doc`, créé ce soir, sans harnais (« citée, non vérifiée »).

**Le recall qui rend la fiche** : `recall_fiches « régime GPU »` en cohérence stricte, `partial:
false`, un résultat, la Fiche « régime GPU » par `vector+bm25` ; `cited_by
sources/2026-10-25-migration.md` rend la fiche avec son apport (`plan.total` 1). Les deux par le
chemin de l'hôte, celui que le client MCP emprunte.

## Ce qu'il faut savoir

- **`cited_by` prend la valeur sans préfixe de genre** (`sources/…`, pas `doc:sources/…`) : la
  Ref range le genre à part. Mon premier appel avec le préfixe a rendu zéro ; la doc du
  paramètre `value` donne pourtant l'exemple préfixé. À corriger dans le graphe.
- **Le client MCP ne voit pas les outils nouveaux** : le serveur mince redemande la liste à
  l'hôte à chaque `tools/list`, mais Claude Code la garde en cache et le serveur ne notifie
  qu'un hôte revenu. Toute session MCP démarrée à partir de maintenant les a ; la mienne non.
- **Un écart dit à l'ouverture** : la ligne RefType « fichier » du 19 octobre n'a pas de
  fichier, elle est née avant eux ; rien ne la régénère tant qu'elle n'est pas réécrite. Memory
  et Subject ne déclarent pas de fichiers : ils ne comptent pas comme écart et ne reviendraient
  pas d'une reconstruction. C'est M4 qui les relira.
- **`put_source`, `put_fiche`, `put_apport` ne sont pas atomiques** (« la ligne, ses chunks, son
  plein texte et ses vecteurs par plusieurs ordres sans transaction »), ce que chaque réponse
  dit. Rien n'a failli ce soir.
- **Un refus net** : `citer_source` sans le genre `doc` n'écrit rien et dit quoi faire ; c'est
  mon script qui avait mal lu la liste des genres (le mot « doc » figurait dans le rappel).
