# Premier arrêt du pilote — liste ELF trop étroite

Porteur cœur C++, [même ticket d’annulation](../tickets/2026-11-09-sous-vingt-ecrivains-un-pool-plein-laisse-une-annulation-inachevee.md).
Premier arrêt respecté, aucun retry ni exécution du pilote, aucune mesure.

## Ce qui a été fait

Unité `cx-coeur-pilote-compile-12nov-01`, wrapper PID 1043954, sous poste lourd,
MemoryMax 8 Gio/swap 0, RuntimeMax170s+arrêt2s, namespace réseau fermé.
Staging : 830 fichiers du paquet02 et leurs empreintes vérifiés, deux générés
copiés avec SHA vérifiés, source pilote inchangé. Compilation **une TU** code0
puis lien code0 ; aucune compilation moteur.

Le contrôleur refuse ensuite la liste DT_NEEDED : `ld-linux-x86-64.so.2` ne
figurait pas dans sa liste prévue (stdc++, m, gcc_s, libc). Ce refus est celui
d’une garde trop étroite, pas un échec de compilation ni une preuve d’une
dépendance moteur partagée. La garde n’a pas été élargie ni rejouée.

L’unité finit code2 en 2,322s, CPU2,293s, mémoire pic325,9M sans swap ; MainPID0,
wrapper absent, poste relu0/4. Aucun processus lourd propre ni mesure conservé.
C7 et arbre prévenus de la libération.

## Binaire conservé, non exécuté

Dossier luciepc :
`~/.cache/rag3db-moteur-notes/annulation/cx-pilote-retrait-0511dece7/`.

- `cx-mesure-retrait` : 25 610 008 octets, SHA256
  `0c868053afea43df3cb2eb5fe8e77f597701b8ab4eb2a0c403ca854ef64cf704` ;
- `pilote.o` : SHA256
  `f964fbacbe8c16f3a72c7325d18eb318041613cde73587101a9c90fb703dd0ef` ;
- recette exécutée : SHA256
  `b4d424cae166f90ba5d3c67aa7a0533bc579bcd17a86c1b973e250e0e618b985`,
  argv exacts dans `compile-result.json`, `link-result.json` et
  `refus-elf-provenance.json` ;
- ELF64 x86-64, aucun RPATH/RUNPATH ; NEEDED : libstdc++.so.6, libm.so.6,
  libgcc_s.so.1, libc.so.6, ld-linux-x86-64.so.2.

Preuves : `unite-compile.log`, stdout/stderr et result.json de compile/link,
`elf-header.stdout`, `elf-dynamic.stdout`, `elf-program-headers.txt`,
`premier-echec.json`, `staging-verifie.json`, `compile-limites.json`,
`refus-elf-provenance.json`, `chargeur-systeme.json`. Aucun compile-manifest
réussi ni résultat d’égalité entrées/survivants : le pilote n’a pas été exécuté.
Le compilateur a émis un warning non fatal sur `advanced` utilisé seulement
par KU_ASSERT en Release dans l’en-tête assorti ; source figée non modifiée.

## Provenance du chargeur, lecture seulement

L’interpréteur du binaire est `/lib64/ld-linux-x86-64.so.2`, résolu en
`/usr/lib/ld-linux-x86-64.so.2`, propriétaire selon la base pacman locale :
`glibc 2.44+r50+g1848099f063e-3`. SHA256 du fichier :
`2a950d46575efa0210d22440988f5a6b4507ebddc6c86d9a42e6e7ef6f01463e`.
Le `transaction_test` sauvegardé sous `binaires-0511dece7/` contient lui aussi
ce NEEDED. Ces contrôles utilisent readelf et la base de paquets, sans exécuter
le chargeur ni le pilote.

## Suite à relire, pas déclenchée

Faire relire à C7 ce seul ajout système à la garde et la provenance du binaire
existant ; aucune extension générique de la liste, aucun relink/rebuild/retry
automatique. Si revue favorable, reprendre explicitement les contrôles adaptés
sur ce même binaire avant de redonner une autorisation de cellule. La première
cellule, ses trois paires et les égalités restent **non mesurées**.
Pas de matrice complète ou de conclusion sur DdlEveryMinute.
