# Core RO : copies privées prêtes à admission

**STAGING SEUL, AUCUNE ANALYSE OU ADMISSION.** Autorisation C7 de préparation concrète après RO favorable Hotfixs sur7ec74865853ad5556dd77dc9bc925c338b293c39. [Recette et bornes](17-preparation-exacte-de-la-lecture-du-core.md) inchangées.

J PC : `/home/lucied/.cache/rag3weaver-build/cx-core-ro-01`. Les six scripts du pilote7ec et la recette ont leurs SHA attendus. Les cinq entrées (GDB, zstd, ELF, header et core compressé) ont été vérifiées puis copiées dans J/inputs, et les copies revérifiées. **Les chemins d'exécution des entrées restent ceux de la recette relue** : les copies préservent leur provenance, sans relocaliser implicitement GDB ni changer sa recherche de données. Un changement d'une entrée effective sera encore refusé avant/après analyse.

- Recette SHA256 `9b0489196b524feabf2e4aca9870d57edd1a4956e872346bf6e232dc5b37bda2`.
- [Reçu PC rapatrié](preuves/core-ro-staging/preparation.json), SHA256 `7c0f0f41edd9771756527d746a0a0aca3651ed7d3570e5f518eb004840d41c6d`, avec tailles, empreintes et chemins des cinq originaux/copies.

Scratch `/home/lucied/.cache/rag3weaver-build/cx-coeur-core-ro` créé neuf, vide. Unité cx-coeur-core-ro-01.service LoadState=not-found/MainPID0/ControlGroup vide ; aucun processus attribué. Ni raw, ni resume.log, ni core décompressé dans cette préparation. Worktree source propre à la copie, HEAD7ec. Aucun GDB/zstd, inférieur, build/test natif, service, poste ou réservation lancé.

Pin soumis à l'admission distincte C7 : `bash /home/lucied/.cache/rag3weaver-build/cx-core-ro-01/launch.sh`,120s total après vrai poste4Gio/swap0/quota effectif, décompression20s/GDB35s inclus, contrôleur80/unité90+5/fin native95/final110/watchdog115+5. Les seuls symboles prévus sont ceux de l'ELF exact, auto-load/solib/debuginfod désactivés avant lecture du core et sysroot privé vide. Aucune garantie de variables DWARF ni conclusion de causalité.

Le GO2400s précédent reste clos, les tickets crash/env/C1 ouverts. Ce staging ne lance pas la commande ; aucune permission supplémentaire n'a été demandée pour ces copies déjà autorisées.
