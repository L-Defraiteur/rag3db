# La sonde des doublons dans les bases de Lucie (13 novembre 2026)

Session hotfixs, à la demande de c7 (l'orchestration), après la preuve du cœur (la première
session du cœur C++). Le texte suppose qu'on n'a rien suivi.

## Pourquoi

Le cœur a prouvé (`PkLookupUnderCheckpoint`, ticket
`docs/coeur-cpp/tickets/2026-11-13-une-recherche-par-cle-primaire-rate-des-cles-validees-pendant-un-point-de-reprise.md`)
qu'une recherche par clé primaire pouvait ne pas trouver une clé déjà validée, pendant un point de
reprise. La crainte : qu'un MERGE ait, en service, créé une seconde rangée pour un nœud qui existait.

## La réponse

**0 doublon dans les quatre bases : ni de clé primaire, ni de clé logique.**

| base | poste | clés primaires en double | clés logiques en double |
|---|---|---:|---:|
| code-lourd | luciepc | **0** | **0** (Scope 82 855, File 7 278, Symbol 84 928, Library 464) |
| code-principal | luciepc | **0** | **0** (Scope 65 914, File 6 314, Symbol 73 373, Library 398) |
| code-rag3db-depot | portable | **0** | **0** (Scope 79 662, File 7 803, Symbol 86 152, Library 448) |
| memoire-longue | portable | **0** | sans objet (pas de table de code ; voir les limites) |

Aucun doublon, donc aucun morceau ni vecteur rattaché à un doublon, et rien à dater.

## Ce que la sonde compte

- **Les doublons de clé primaire.** Pour chaque table de nœuds :
  `MATCH (n:T) WITH n.<clé> AS k, count(*) AS c WHERE c > 1`. C'est une agrégation sur les rangées
  elles-mêmes, qui ne passe pas par l'index de clé. Le cœur a expliqué pourquoi on s'attendait à 0 :
  le moteur refuse d'écrire une seconde rangée de même clé (« duplicated primary key », vérifié par
  l'index à l'insertion), et un écrivain ne tourne pas pendant le point de reprise. Le raté ne touche
  qu'un lecteur.
- **Les doublons de clé logique, le chiffre décisif.** Si un doublon est né, c'est côté rag3weaver :
  une lecture qui rate la clé, puis un CREATE sous **un autre uuid**. La sonde regroupe donc les
  rangées par clé logique :
  - Scope par `key` (par exemple `file#benchmark/serializer.py#base_dir:variable`) ;
  - File par `path` ;
  - Symbol par `_project` et `name` ;
  - Library par `_project` et `import_path`.

  Pour chaque doublon, elle donne chaque rangée avec son `_uuid`, son `_snapshot`, son
  `_absent_since`, et le nombre de ses morceaux (`T_CHUNKED_FROM`) qui portent un vecteur. Ces
  doublons comptent dans son code de sortie.
- **Les clés sont remplies.** Sur code-rag3db-depot, Scope compte 79 662 rangées, 79 662 `key` non
  nulles, 79 662 `key` distinctes et 79 662 `_uuid` distincts. Un `key` nul partout aurait aussi
  donné 0.

## Le témoin rouge

Le cœur ne peut pas fournir de base à doublon : son test ne crée jamais de seconde rangée. Le témoin
est donc fabriqué (`outils/temoin-doublons.cpp`). C'est une base neuve avec deux Scope de même `key`
sous deux uuid, dont l'un porte un morceau. La sonde y compte 1 clé logique en double et rend le code
1 : « rangée 0, _uuid u1, … morceaux 1 » et « rangée 1, _uuid u2, … morceaux 0 ». Le même programme
montre que le moteur refuse un troisième CREATE sous l'uuid `u1` (« Found duplicated primary key value
u1 »).

## Ce qui a été lu

Des copies reflink des instantanés `avant-835d5298f`, pris par G hôtes arrêtés : à luciepc à 20 h 06
(code-lourd, code-principal), au portable à 20 h 07 (memoire-longue, code-rag3db-depot). Les copies
sont sans `.wal`, et `fuser` était vide. Lecture seule, par la lib
`rag3db-build-835d5298f80b832377a7caa80ee58d242e3ece23-Release-p12-v11-g17-s18-rcOFF-stOFF-asanOFF-tsanOFF-ubsanOFF`.
À luciepc, la sonde a tourné sous `poste lourd` avec `POSTE_MEM_MAX=32G`. Rien n'a touché les bases
de Lucie elles-mêmes. L'état lu est celui de 20 h 06 : une écriture faite depuis par les hôtes n'y est
pas.

## Les limites

- **memoire-longue** : seules ses clés primaires sont contrôlées. Je n'y connais pas de clé logique
  (fiche, apport, référence) qui permettrait de reconnaître deux rangées comme le même objet sous
  deux uuid.
- **Les clés logiques choisies** sont celles qui nomment un objet du code. Une autre table, ou une
  autre forme de clé, n'est pas couverte.

## Rejouer

```bash
ssh -T lucied@luciepc 'bash -s' <<'EOF'
N=~/.cache/rag3db-tickets-notes        # sonde-doublons.cpp y est, et son binaire
S=<dossier de copies neuves>; b=code-lourd
cd ~/.cache/rag3weaver-build && POSTE_MEM_MAX=32G ./poste lourd timeout 3600 $N/sonde-doublons $S/$b/$b.rag3db > $S/$b-doublons.txt 2>&1
grep -E 'DOUBLONS|^LOGIQUE|^BILAN|^ERREUR' $S/$b-doublons.txt
EOF
```

Code de sortie : 0 sans doublon, 1 avec des doublons, 2 si une requête échoue. Relevés : à luciepc,
`~/.cache/rag3db-locks/sonde-orphelins-835d/{code-lourd,code-principal}-doublons.txt` ; au portable,
`~/.cache/rag3db-locks/sonde-orphelins/doublons-{memoire-longue,code-rag3db-depot}.txt` et
`temoin-doublons-3.txt`.

## Une leçon du même soir : `run_e2e.sh` prend le poste lui-même

Pour le rouge de `e2e_catalogue_gabarits`, j'ai d'abord lancé `poste lourd … ./run_e2e.sh …`. Les
trois passes ont refusé de partir en une seconde, avec `ROUGE validation : aucun plafond effectif <= 32
Gio`. La raison :
- `run_e2e.sh` relance la passe par son confineur, `extension/rag3weaver/scripts/confiner_validation.py` ;
- le confineur prend lui-même le poste, avec `POSTE_MEM_MAX=32G`, sauf s'il le trouve **déjà tenu** ;
- dans ce cas, il vérifie que le cgroup courant est plafonné à 32 Gio au plus.

Le `poste lourd` extérieur tenait le poste avec un plafond plus haut : refus. **On lance
`./run_e2e.sh` directement, sans `poste lourd` autour** ; il attend sa place dans la file comme les
autres.
