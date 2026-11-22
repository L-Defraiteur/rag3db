# Le processus des tests de la lib meurt en SIGSEGV à sa sortie, tous les tests verts

- **État** : ouvert (22 novembre 2026, écrit par G)
- **Pour** : propriétaire à départager : lucivy 4.4.1 (un fil de fusion vivant à la sortie) ou le démontage du moteur
- **Famille** : produit (la sortie d'un processus qui a ouvert des bases)
- **Gravité** : moyenne. Un lot vert est rendu rouge (2 999 tests passés, 0 échec) ; en service, un hôte qui s'arrête
  pourrait mourir de même à sa sortie.
- **Atteignable en service** : peut-être (à l'arrêt d'un hôte qui a ouvert des bases), non prouvé
- **Touche rag3weaver** : à départager

## Ce que c'est

Le 22 novembre 2026 à 20 h 51, la passe du pousseur de `judge-grant-said-2` (archi, une ligne de `gate.rs`) rend la
lib rouge : 2 999 tests passent, aucun n'échoue, puis le processus de tests
`target-pusher/debug/deps/rag3weaver-d9e441043d7b65a7` (pid 3584128) meurt en SIGSEGV (signal 11) à sa sortie. Lib
6929acb98d, lucivy 4.4.1. Archi voit des résidus `Lucivy_*` juste avant.

## Ce que le core dit

Le core est tronqué par systemd-coredump (8,5 Mo gardés sur 32 Gio d'espace virtuel) : aucune pile symbolique. On en
tire pourtant :

1. **Le fil fautif est créé tard** : LWP 3672083, le plus récent des 37 fils, lancé peu avant la sortie (un fil de fond
   ou de fusion, pas un fil de test).
2. **Il saute dans le tas** : le PC 0x55efa81cb9e0 est hors du binaire (base 0x55ef65d99000, taille 0x1040d000,
   décalage 0x424329e0). C'est un appel par un pointeur de fonction ou de vtable pendant.
3. **Un pointeur est écrasé par une chaîne** : gdb bute sur 0x61775f6b636f6c67, soit les octets ASCII « glock_wa ».
   C'est la signature d'un objet libéré puis réutilisé dont on appelle encore une méthode.

## L'épreuve qui départage

La montée de lucivy en 4.5.0 (`lucivy-4-5-0-2`, en preuve chez lucivy), qui corrige un fil de fusion vivant au
démontage. Si le SIGSEGV de sortie disparaît sur plusieurs passes de la lib après elle, le ticket se ferme sur cette
cause. S'il revient, la pile est à prendre sur un core entier : `ProcessSizeMax` et `ExternalSizeMax` de
`/etc/systemd/coredump.conf` relevés à luciepc (réglage système, à Lucie).

## D'ici là

Un lot rendu rouge par ce seul SIGSEGV de sortie (tous les tests passés) se rejoue une fois tel quel.
