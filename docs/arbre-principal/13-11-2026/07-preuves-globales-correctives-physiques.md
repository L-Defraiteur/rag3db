# Qualification runtime corrective physique — v2

Application EXACTE `4e5ae8a324f50313b74f70772eb2e92ee41fad6f`, pilote EXACT
`a18f8006d052f1fb57ff88cfd5d38965ea3ce179`, GO C7. Runtime, superviseur et
finaliseur VERT. Ce résultat qualifie cette portée ; aucun GO master, migration
legacy, projection de coordonnées fichier, modèle dense ou base réelle déduit.
Les deux essais précédents restent ROUGES, jamais convertis rétroactivement.

## Provenance et temps

F8 8/8, vrai minimal et compilations sont hérités du premier run source4e,
ledger5e98111915438ee3e1ab03e74cef5413ab36a35a9735c3f3529dfa4fc6809078.
Aucun Cargo/F8/min/rebuild dans les correctifs. Nouveau ledger v2 publié
AVANT runtime, quatre ELF copiés et SHA vérifiés :
`419ea1736b36174d1d0ab55d191e80d301a1cf02b4ff8a7930f69fef3421c36f`.

Départ correctif initial898110.830219227 conservé. Reçu v2 neuf3024s après
poste réel ; échéance effective901707.556114395, sous borne absolue
901710.830219227. Finaliseur :1275.019330s depuis départ initial, environ
702.245s depuis driver v2. Aucun reset3600. Première fenêtre6000 expirée.

## Résultats réellement observés

| Portée | Résultat | Temps pilote |
|---|---|---|
| Lib complète depuis crate |2299 fonctions passées,0 échec,66 ignorées|49.98s|
| NAV réel Chromium |40/0/0,quiescence vide|138.09s|
| CLI preflight |5/0/0|0.14s|
| Lectures flag ancien |27 brutes,26 jugées,1 non jugée explicite|9.96s|
| CREATE exact flag retiré |1/0/0,même ELF/profil contrôlé|1.32s|
| Hôte |13/0/0|12.61s|
| PostgreSQL |36/0/0|8.66s|
| TX ligne à ligne |3 fonctions brutes passées|39.13s|
| TX merge/copy |1 fonction brute passée|1.11s|
| TX arrêt par paquet |18 fonctions brutes passées|150.73s|
| TX vecteurs après rollback |3 fonctions brutes passées|14.36s|

Chaque suite e2e réutilise les primitives réelles et joue ses deux filets
séparément. Les comptes bruts ne prouvent pas autant de juges indépendants
(helpers enfants inclus). Les66 ignorés lib ne sont pas des réussites.
Publication14 et symboles12 sont inclus dans la lib, sans seconde invocation.

Physique :20 fonctions brutes/19 juges/1 helper et8 morts d’enfants,
sept états batch dont avant/après deuxième commit, deux bootstrap. Assertions
occurrences8/contenus6/références8/modèlesdenses0, préfixe persistant avant
reprise, sources retirées, fermeture/réouverture et recherches passées.

Lectures :27 juges distincts/28 invocations brutes (ancien26+CREATE1).
Non-jugé explicite conservé, aucune qualification du format207701. Profil
capturé comparé avant et sur processus CREATE, seul ANCIEN retiré, garde
root runtime-disk exacte et primitives état/résidus maintenues.

## Ressources et limites

Max32GiB, HIGHmax, quatre rôles BURN CPU, jobs8, pool global1GiB conforme
à la garde IndexNode, pas minimum moteur universel revendiqué. Événements
memory.high/max/oom/oom_kill restent0 ; pic mémoire non capturé par cette
recette, ne pas en inventer un. Target du -sb final8812176962octets (<32GiB).
130 relevés maps runtime SDK29, plus attestation
backend main. SDK29 SHA
`b014febe51417ca535218d1c0b18308e7d301157abf45027a5cbba70dfc54929`.

Devices :7 observations sur4PID des chemins /dev/dri/renderD128/129/130.
Pas de preuve de charge GPU depuis ces FD, et aucune absence d’ouverture GPU
revendiquée. observations-protegees=[] pour cette portée ; règle générale
UNKNOWN fd/exe/environ/maps ≠ observation négative, notamment devices=[].

Créneau BGE7879/card2 réellement accordé C7, santé avant/après bge-m3 dim1024
sparse=true factice=false ; usage lectures/CREATE uniquement puis rendu
explicite08:00:56 locale. Aucun redémarrage/modèle changé, connexions baseline
non arrêtées. Les autres suites ont poursuivi sans ce créneau.

Nouvelle PG `cx_phys_2191_20261113_u1d`, OID1989866, absence préalable puis
création unique ; OID/propriétaire/commentaire réattestés avant dump,0 session.
Dump-Fc25624octets SHA
`7f574a972b2ea6d1954938459f193ca24e736a8f0f17a94b34b845753d6deea4`.
PG u1/u1c/u1d conservées, aucun DROP/reset ; identité fraîche requise avant
futur retrait. Fixtures des rouges conservées.

Superviseur validation-swv_vplx VERT/code0/restes[]/nettoyé, bus
/tmp/r3v-t3ilthb7 absent. Quiescence NAV/finale live[]/context[], scan
finaliseur vide, TMP NAV possédé retiré, quatre ELF réhashés. Source4e et
pilotea18 restés propres/figés sur PC.

## Preuves conservées

PC : cx-passe-raccord-physique-union-complement-v2 et
cx-raccord-physique-global-complement-confinement-v2/validation-swv_vplx.
Copie locale : cx-preuves-raccord-physique-union-complement-v2-local.
1064 fichiers identiques SHA sur les deux postes ; manifeste SHA
`b57a8d8749a7d3dab2ce38c49dfe7355662144a762271587d14727dc63f1ab34`.
Rouge correctifv1 :2843 fichiers conservés et vérifiés SHA deux postes,
rapport05 ; rouge initial également recopié après arrêt avec son superviseur,
2026 fichiers SHA identiques deux postes.

La précondition CWD initiale est corrigée ; le défaut profil512MiB est
corroboré par contrat et graphe concurrent, sans attribuer ce graphe au juge
FAILED. Ce juge passe en v2, ce qui ne recrée pas la panique non capturée.

## Quatre ELF réattestés

- lib : `9c1e23c88abc87ebea6188d99e4eb65ec7c3a0533f1aaf3f152d545fc9867e92` (466516040 octets), source4e et223gabarits.
- navigateur : `f14efb4d5b58c50707b47852e90a6470d590f63bf59c57caff5a5b556cdafcc0` (466646640 octets), source4e et223gabarits.
- cli : `98832bdfe6200f1ba7cc61c972e91f4e8450c83f9f46cb1e8fbfb6097088fd01` (461467448 octets), source4e et223gabarits.
- backend : `b68721ae0f8e4d69ade07107736537690458f335d98b1232f70d10891a4ed0fb` (422915632 octets), source4e et223gabarits.
