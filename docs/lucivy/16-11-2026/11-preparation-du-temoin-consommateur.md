# Témoin consommateur et cœurs sur octets — baseline NON exécutée

Même ticket : `docs/lucivy/tickets/2026-11-16-indexed-false-est-ignore-pour-un-champ-texte.md`.
C7 a levé la dépendance de conception après RO Recherche pour autoriser cette
PRÉPARATION source/test/recette, sans builder natif, sélection ni nouvelle passe.

## Source publiée avant exécution

Lucivy `81b0e74f0254c8216864f90a460f8969842432ed`, branche publiée
`cx-lucivy-corpus-admission`, worktree portable
`/home/lucied/git_workspaces/cx-lucivy-corpus-admission`. Parent890, aucun code
produit touché : six fichiers dans tests/, dont trois nouveaux. Aucune commande
Cargo/rustc/rustfmt ou exécution d’ELF jouée pour ce candidat.

- `common/derived_fixture.rs` contient le panel en QueryConfig et l’entrée
  `with_derived_fixture(load, build)`. Le VRAI test derived_in_ram utilise cette
  entrée ; toute construction/native writer/scratch reste dans sa closure build.
- La même entrée sert au témoin mémoire :50 documents `return 0;`, sans
  couverture du panel, compteur de builder. **Baseline délibérément non corrigée** :
  après chargement/décodage, elle appelle encore build sans vérifier le panel.
  Le témoin doit donc montrer compteur1 au lieu de0, pas une API absente.
- Neuf cas sont déclarés dans `test_derived_fixture_admission.rs`. Un seul est
  proposé pour la première exécution rouge. Les huit autres ne sont pas verts
  par lecture de code : erreurs de loader/UTF8, Unicode/backmap, séparateurs et
  frontières, regex nonvide, définition fuzzy existante, decode-error≠nohit.
- Les QueryConfig et leur ordre/labels ont une transcription statique identique
  aux onze Q du parent890 : [preuve](preuves/derived-admission-source-preparation/panel-transcription.json).
  Ce n’est pas une preuve de compilation ni d’exécution du constructeur Rust.
- `common/corpus_oracle.rs` extrait literal/fuzzy/regex et le filtrage de
  frontières sur octets immuables, sans accès FS. Les erreurs UTF8 ne produisent
  pas une liste de hits vide. Unicode/backmap restent ceux du cœur extrait ;
  fuzzy réemploie fuzzy_spans, distinct du chemin index, pas indépendant de sa
  définition. Regex reçoit une RustRegex précompilée et ignore les spans vides.
- Trois wrappers ground_truth réutilisent les cœurs ; ils conservent leurs
  lectures, continue et conversions lossies historiques. L’ancien wrapper de
  frontières reste tel quel. **Ces enveloppes ne sont PAS l’admission.** Le
  loader natif corpus conserve également ses filtres/continue de sélection en
  baseline : le mock d’erreur typée ne prouve pas que cette lecture réelle a
  déjà été rendue fatale. L’étape de correction devra traiter explicitement les
  octets sélectionnés/épinglés, après le vrai rouge ; ne pas la déclarer livrée.

Aucun changement de common::usable/ABSENT_OK, du corpus ou de son ordre ;
aucun retrait de l’assertion de référence nonvide, de documents/spans, de
sidecars/taille/métadonnées ni de la réouverture. Les dépendances et API produit
restent identiques. La baseline ne constitue pas un correctif de couverture.

## Recette du rouge proposée — aucun GO d’exécution déduit

WPC prévu `/home/lucied/git_workspaces/cx-lucivy-corpus-admission-16nov`, APP81b0.
Q proposé `/home/lucied/.cache/rag3weaver-build/cx-lucivy-derived-admission-red-16nov-v1`.
Même target privé Tnonce3552/inode164536077/dev55, markerSHA5a1671. Pas un nouveau
T. Dernier Commoncore wbvzcmca3528029/start119072861 TERMINE101/restes[]/nettoyé,
ELFcore rouge106fa398fcc24296c1d4b94198a150ed87917fac66e46531af5c617444fcf9a2
à copier/rehash avant compilation. Nouvelle autorité/logs/nonce et relevés frais,
UNKNOWN nonattribués conservés, owned/attribués bloquants ; jamais ROOT/proc universel.

Pilote proposé `outils/qualify-derived-admission-red.py` SHA
99a8afcacfabd76928e596f956345dbe786641cbd96baf8c86dae32c69e30db1 ;
admission `outils/admit-derived-admission-red.py` SHA
7649a9b9d34ec3cdfc1333f33bc216f260fbda9a6d52f3c70fe979d7aabff5e2.
Ces sources sont publiées ici avant toute exécution. Syntaxe AST vérifiée
seulement ; le paquet/preflight exacts seront préparés puis publiés à leur tour.
Pilote partage uniquement les contrôles metadata du core2fa59 et du rouge0a269,
Common6439/poste79e3 inchangés, pas de duplication du classifier /proc.

Budget proposé **600 secondes uniques après vrai poste**, mémoire32G/HIGHmax,
swap0/quotaCPU400%/jobs<=4 et deux cœurs libres selon garde hiérarchique.
Parent cwdQ OBLIGATOIRE avant lancement ; pas de shell fish `set -eu` : enveloppe
bash explicite pour le prochain lancement. La commande candidate après
assemblage/RO puis GO distinct serait :

```bash
cd /home/lucied/.cache/rag3weaver-build/cx-lucivy-derived-admission-red-16nov-v1
POSTE_MEM_MAX=32G POSTE_SWAP_MAX=0 POSTE_CPU_QUOTA=400% RAG3WEAVER_BUILD_MEMORY_HIGH=0 /home/lucied/.cache/rag3weaver-build/cx-lucivy-derived-admission-red-16nov-v1/support/poste lourd timeout --signal=TERM --kill-after=5s 610 python3 /home/lucied/.cache/rag3weaver-build/cx-lucivy-derived-admission-red-16nov-v1/admit-derived-admission-red.py
```

Étapes strictes sous la même échéance :

1. Recontrôler paths/UID/dev/inode/nonce/copies/SOURCE propre/ancien reçu/ELFcopié,
   puis Common neuf pour le reste moins2s, pas de reset.
2. Compiler offline release, sans exécution, les trois targets
   test_derived_fixture_admission, test_derived_in_ram, test_sfx_v3_ground_truth.
   Ceci vérifie aussi le raccord et l’extraction à la compilation ; **aucun**
   corps de test natif ni construction d’index n’est lancé.
3. Exiger exactement ces trois artifacts attestés sous T. Écrire/fsync le
   ledger APP+ELF+SHA AVANT tout appel d’inventaire. Inventorier les trois et
   leurs ignorés, puis écrire/fsync l’intégralité du ledger d’inventaire AVANT
   l’unique témoin. Corrige le protocole du ledger tardif de la passe précédente.
4. Jouer uniquement `test_derived_fixture_admission --exact
   missing_panel_is_rejected_before_any_builder --test-threads=1 --nocapture`.
   Rechercher running1, rc101,0pass/1fail/0ignored, assertion compteurleft1/right0
   et diagnostic d’admission. Un défaut de compilation/API ou un autre échec
   ne serait PAS ce rouge discriminant. Les huit autres cas restent NON JOUÉS.
5. Préserver le log natif/identité/SHA, attenduROUGE101, scratch vide,
   markerT inchangé, scans frais puis Common terminé/nettoyé/owned absents.
   Première anomalie inattendue STOP sans retry. Un résultat attendu rouge
   ne qualifie pas le core et n’autorise pas encore son correctif ou un vert.

Pas de corpus réel lu par le témoin, pas de .env/.vault, pas d’index/base/service,
pas de rejouage des anciens trois warnings, pas de nativecase/corpus7.2/ULP,
PR42/main/tag. NeufHOLD et ignorés du profil core restent nonqualifiés.

RO Recherche demandée sur ce NOUVEAU source81b0, distincte de la RO déjà reçue
sur la conception. Assemblage/preflight metadata peuvent avancer en parallèle ;
aucune compilation ou exécution avant publication complète et GO exact distinct.

## Paquet metadata assemblé après publication des sources

Q réel UID1000/dev55/inode168475170, nonce neuf
`2e5cac17050a7d5dcf4ea8a932309625`. WPC créé sur branche propre
cx-lucivy-corpus-admission-16nov à81b0 ; aucun Cargo ni ELF lancé. Les métadonnées
seules ont été vérifiées après publication1162dc7f4d des sources du pilote/recette.

[Préparation et empreintes](preuves/derived-admission-source-preparation/SHA256SUMS).
Préflight exact SHA3557847be05f314650dcb2afb9f5ced24c57de84c36ce959cfd161f3e642b76e.
Les neuf empreintes de copies incluent pilotes99a8/7649, helperscore2fa59/rouge0a269,
Common6439/poste79e3 mode755, ELFancien106fa copié RO, ledger des six sources et
transcription des onze configs. Aucun ancien ELF adopté sans nonce/owner/source.
Target unique3552 inchangé, taille RO mesurée1754562560octets, pas de purge.

Précontrôle frais sauvegardé AVANT assertion :11UNKNOWN nonattribués conservés,
aucun owned/attribué actif. Reçu core ancien TERMINE101/restes[]/nettoyé validé.
Scope/logs neufs **absents**, scratch privé vide. Les sources et le raccord sont
noncompilés ; aucune observation du rouge attendu n’est encore revendiquée.

Pour la future invocation distante, utiliser explicitement **bash**, avec le
shell parent déjà dans Q. Cette enveloppe corrigera le diagnostic fish antérieur,
sans répéter un lancement ; elle est proposée ici, **NON lancée** :

```bash
ssh -o BatchMode=yes luciepc 'cd /home/lucied/.cache/rag3weaver-build/cx-lucivy-derived-admission-red-16nov-v1
exec bash -s' <<'BASH'
set -euo pipefail
set -C
test "$PWD" = /home/lucied/.cache/rag3weaver-build/cx-lucivy-derived-admission-red-16nov-v1
POSTE_MEM_MAX=32G POSTE_SWAP_MAX=0 POSTE_CPU_QUOTA=400% RAG3WEAVER_BUILD_MEMORY_HIGH=0 /home/lucied/.cache/rag3weaver-build/cx-lucivy-derived-admission-red-16nov-v1/support/poste lourd timeout --signal=TERM --kill-after=5s 610 python3 /home/lucied/.cache/rag3weaver-build/cx-lucivy-derived-admission-red-16nov-v1/admit-derived-admission-red.py > launch.log 2>&1
BASH
```

Le noclobber protège aussi un éventuel journal existant. GO exact600 distinct
encore requis après RO du nouveau SOURCE81b0 et du paquet99a8/7649/preflight3557.
La RO de conception ne qualifie pas ce nouveau code, son compilation ou son
pilote. Banc est sollicité pour la recette/ledger/deadline/provenance seulement,
Recherche pour le raccord/modes/extraction/discriminant ; aucun run par eux.
