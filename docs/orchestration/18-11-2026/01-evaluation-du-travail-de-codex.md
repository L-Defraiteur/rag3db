# L'évaluation du travail de Codex (16–17 novembre), rassemblée (18 novembre 2026)

Lucie, 18 nov. 11 h 3x : « c'est utile tout ce qu'il a fait ? oui lançons tout ça et d'abord chaque session
évalue la pertinence de ce qu'a fait Codex ». Chaque session a relu, dans son périmètre, les 76 commits que
Codex a posés sur master (927941a67a..99fef92ce6) et les branches `cx-…` de son rôle, et dit ce qui est utile,
ce qui est à requalifier, ce qui est inutile ou nuisible. Les pages sont sous `docs/<session>/18-11-2026/01-evaluation-du-travail-de-codex.md`.
Ouverte à 11:41, remplie à mesure.

## Ce que l'orchestration a vérifié elle-même

- Identité et attribution : 76 commits, tous `luciedefraiteur@gmail.com`, aucune mention d'assistant.
- Bascule de lib : jouée par le rôle G de Codex le 16 à 11 h 1x (806715b2d6, épingle 68590270e2) ; les deux
  hôtes répondent `rag3db-build-59ac9c86bc…` à `/sante` ; la garde au cache est égale à master
  (push_garde fff9a4d9e20e739e, passe_epinglee 2150fcd847b1df7d) ; `poste` inchangé (79e3a8bbf457e3bc).
- CI : « CI Workflow » vert sur 99fef92ce6 ; sept runs amont (LDBC, ClickBench, SimSIMD, Multiplatform)
  lancés par Codex le 17, en file sans runner ; l'annulation est à Lucie (refus du classifieur ici).
- Forme : messages de commit d'une ligne sans corps ; passation d'orchestration en jargon non défini
  (coupons, GO, END, ownedquiet), qui parle surtout de qualifications et peu de ce que chaque liste a gagné.

## Les verdicts des sessions

| session | page | avance vraiment | papier | gêne | effet sur la liste du 16 |
|---|---|---|---|---|---|
| memoire | 46911155a | le « Text file busy » de codex_threads : correctif juste et meilleur (un enfant /bin/sh écrit, le parent ne tient aucun writer), rouge vu, lib 2605/0 ×2 sur 59ac ; **à garder** | six pages, 23 branches cx-memoire-script-fixtures-*, des JSON d'audit pour un correctif d'une heure | **`~/.cache/rag3weaver-build/garde_worktree.py` écrasé par scp à luciepc avant empreinte, ancienne version perdue** ; deux tickets de nettoyage restés en branche ; un vert cité pointe un dossier qui ne le contient pas | 1/14 faite ; 2.2 à 2.13 intactes |
