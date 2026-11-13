# Un lecteur plein texte retient des écritures de blobs

- **État** : ouvert ; ciblé privé P14 qualifié sur a43c475f9 (preuve liée), qualification globale de l’union et livraison attendues.
- **Famille** : produit.
- **Gravité** : tentative de persistance en lecture seule ; la connexion native refuse, mais le lecteur ne doit pas préparer ces écritures.
- **Atteignable en service** : oui, lecteur Catalog BlobBacked ; tentative observée sur fixture synthétique privée, pas de persistance réussie prouvée.
- **Touche rag3weaver** : oui, initialiser_en_lecture, ouverture FTS et CypherBlobStore.
- **Pour** : arbre-principal ; revues mémoire/recherche/orchestration.

Parent : [raccord physique](2026-11-12-migrer-les-ancres-de-code-vers-le-contenu-partage.md).

Preuve P10 e100737d0 : physical-07.log contient BufferedBlobStore dropped with
20 unflushed blobs, refus write dans read-only DB. Le catalogue lecteur monte un
buffer invisible à blob_buffer ; open_with_storage peut retomber sur CREATE et
le handle writable commit à sa fermeture. CypherBlobStore::prepare tente aussi
un ALTER sur toute première lecture. Aucun effet persistant réussi n'est prouvé.

Périmètre autorisé : store RO sans buffer ni DDL, écritures refusées avant query,
handles BlobBacked ouverts par snapshot public lucivy4.3.3 sans écrivain
intermédiaire, aucun fallback CREATE en RO. Fichiers absents/corrompus refusés.
Lecture eager et paquet mémoire transitoire : coût assumé pour ce chemin, sous
limite de qualification32GiB. Pas de modification lucivy/SDK/format persistant.
Modes fichiers/sparse hors preuve native BM25 actuelle restent à qualifier
séparément ; ne pas déduire leur absence d'effets de ce seul témoin.

Qualification attendue : CPU aucune query d'écriture même à Drop ; natif handles
is_read_only, commit refusé, index absent non créé et fichiers DB inchangés après
recherche/fermeture. Le garde de lecture publique sur versions internes reste
strict ; la maintenance interne suit les stockages physiques déjà entretenus.

Qualification ciblée P14 sura43c475f9 : VERTE (voir bilan parent lié).
Créations/écritures/reprise ou lecteur RO réellement éprouvés dans le profil
synthétique BM25. Qualification générale et livraison non faites ; ticket ouvert.


## Reprise corrective distincte — décision C7 après expiration

L’échéance initiale 896572.526453239 est expirée : aucune reprise ni prolongation
de ce reçu. Le rouge global initial reste conservé. Source application EXACTE
4e5ae8a324f50313b74f70772eb2e92ee41fad6f et quatre ELF du ledger inchangés.
Pilote préparé hors WT source, étape corrective DISTINCTE de 3600 s après poste
réel, runtime complet avec cwd crate, sans Cargo/rebuild/F8/min déjà verts.
PG initiale cx_phys_2191_20261113_u1 conservée, identité/OID et dump requis avant
tout retrait ; nouvelle PG privée cx_phys_2191_20261113_u1c, absence préalable
et création unique contrôlées. Aucun DROP/reset. Revue et GO exact requis avant
lancement. Aucun résultat runtime global vert ; P14 ciblé seul demeure qualifié.
Les champs protégés UNKNOWN ne prouvent pas une absence d’accès GPU.
Voir [recette corrective](../13-11-2026/04-recette-corrective-runtime-physique.md).


## Étape corrective distincte : arrêt au premier nouveau rouge

C7 a donné GO exact au pilote e8721a5683166e8ff688e55977bc79cdcf50de11,
application figée 4e5ae8a324f50313b74f70772eb2e92ee41fad6f et quatre ELF hérités.
Portée DISTINCTE de 3600 s : départ monotone 898110.830219227, échéance
901710.830219227 ; aucune prolongation/réinitialisation de l’ancienne portée.
Aucun Cargo/F8/min/rebuild exécuté. Ledger neuf publié et quatre ELF copiés
avec SHA vérifiés AVANT runtime :
7fe4942919dd2ff3fede289c010be11509b5b1fb026b61c7c1e704bf5b9d209a.

Lib depuis le crate : ancien juge CWD passé ; nouveau premier FAILED :
daemon_host::atelier_fichiers::tests::the_workshop_reads_a_scope_from_the_index_of_an_attached_worktree.
Arrêt immédiat, durée portée 85.917 s, runtime environ 8 s. Aucun résumé
lib complet, aucun corps de panique publié avant arrêt : cause précise NON
attribuée. Ni rejeu inchangé ni Cargo pour récupérer une panique. NAV/CLI/
lectures/BGE/PG-suites/TX non lancés. Le rouge global reste ROUGE.

Superviseur validation-ej1zgwoe : ROUGE/code1/restes[]/nettoyé ; bus
/tmp/r3v-py3oi02g absent. Scan RO exe/cwd target+deux WT : live[].
Les UNKNOWN restent non concluants sur absence GPU. PG u1 conservée et
PG u1c créée une seule fois, OID1989865 ; aucune suite PG jouée, aucun DROP.
Fixtures restantes conservées, sans ouverture native ni nettoyage.

Preuves PC : cx-passe-raccord-physique-union-complement-v1 ; copie locale
cx-preuves-raccord-physique-union-complement-v1-local/client : 2843 fichiers
identiques SHA au manifeste après arrêt. Superviseur aussi copié localement.
SHA lib.log : 8295e630e40de1f29d5b0a7df6eae5643cdb865e69722a66ce004acbbf3b71a5.
C7/ED poursuivent attribution RO ; aucun défaut production encore établi.
P14 ciblé reste vert dans sa portée ; aucune qualification globale obtenue.


## Qualification corrective v2 obtenue — livraison en attente

GO C7 exact pilotea18/application4e, lib2299/0/66,NAV40,CLI5,
lectures27jugesdistincts/28bruts,CREATEexact,hôte13,PG36 et4TX passés ;
superviseur/finaliseurVERT sansrésidu, échéancecorrective conservée.
1064 fichiersSHAidentiques deuxpostes. AucunCargo/rebuild ; deuxrouges
antérieurs conservés. PasGOmaster, tickets ouverts avant livraison/revue.
FD /dev/dri observés : aucune absence d’ouverture GPU revendiquée.
Détails, compteshelpers, bornes et dump dans
[preuves07](../13-11-2026/07-preuves-globales-correctives-physiques.md).
