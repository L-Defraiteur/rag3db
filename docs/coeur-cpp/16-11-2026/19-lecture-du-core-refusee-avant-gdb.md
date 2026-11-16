# Lecture du core refusée avant GDB

GO distinct C7-CORE-RO-7EC-16nov120, pilote7ec74865853ad5556dd77dc9bc925c338b293c39 et recette9b0489196b524feabf2e4aca9870d57edd1a4956e872346bf6e232dc5b37bda2 inchangés. **ROUGE sur la première commande de décompression ; aucune analyse GDB acquise.**

Vrai poste4Gio/swap0/quota500000/100000, POSTE_FILS5/admitted_threads5. Parent run-p154081-i151063688.scope, départ monotonic1194772,945472579. Unité cx-coeur-core-ro-01.service lancée ; préflight cinq entrées/limites/absence réseau accepté, aucune entrée modifiée.

zstd code1, timeout=false, durée0,001478s, PID154129 attendu/reapé, groupe/cgroup sans résidu. Stderr brut : Incorrect parameter: --output=/home/lucied/.cache/rag3weaver-build/cx-coeur-core-ro/core, puis usage montrant -o OUTPUT. Le pilote avait construit une option non acceptée par l'outil épinglé. [Ticket ouvert avant correction](../tickets/2026-11-16-la-commande-de-decompression-du-core-utilise-une-option-refusee.md).

Contrôleur arrêté en0,031665s. Entrée fin0,151556s, unité failed/Result=exit-code/MainPID0/ControlGroup vide, quiescent=true, aucun PID attribué restant. Rehash des cinq entrées après quiescence conservé. Résumé réel ROUGE. Poste rendu, GO120clos ; aucun retry ou correction pendant la passe.

**GDB non lancé**, décompression non achevée, hash du core privé17Go non acquis dans cette portée. Aucun inférieur/test/build/service/base/SDK lancé ou modifié. Ce refus d'option n'ajoute aucune connaissance de la cause du SIGSEGV et ne ferme ni son ticket ni C1.

[Lecture et empreinte d'archive](preuves/core-ro-01/lecture.json), archive recus-refus-zstd.tar.gz : recette, commande réelle, stdout/stderr séparés, code/attente, admission/limites/refus/quiescence, rehash et inventaire. Quinze fichiers bruts vérifiés localement après rapatriement ; paquet complet avec copies des outils/entrées conservé aux deux postes. Aucun journal historique réécrit.

Suite proposée seulement après ticket : corriger isolément le constructeur avec l'option -o documentée par le stderr déjà acquis, témoin CPU du vrai vecteur de commande rouge avant correction puis vert, publier recette/RO. Tout nouveau lancement exigera chemins et GO distincts. Aucun appel zstd/GDB supplémentaire autorisé par ce document.
