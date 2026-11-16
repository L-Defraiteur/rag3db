# Le contrôle du dictionnaire compare un compteur à lui-même

- **État** : ouvert (16 novembre2026), constat de lecture seule, hors correctif des offsets
- **Famille** : moteur
- **Gravité** : diagnostic manquant, un contrôle ne peut détecter une incohérence
- **Atteignable en service** : à établir selon les appels de DictionaryChunk::sanityCheck
- **Touche rag3weaver** : indirectement, par les diagnostics internes du moteur
- **Signalé par** : orchestration C7 pendant revue du rouge des offsets
- **Pour** : cœur C++, après coordination et témoin propre

DictionaryChunk::sanityCheck, src/storage/table/dictionary_chunk.cpp, retourne offsetChunk->getNumValues() <= offsetChunk->getNumValues(). La condition est toujours vraie pour ce compteur entier. Aucun ticket correspondant trouvé dans les tickets cœur/hotfixs/closed lors de cette reprise.

Constat séparé du [refus d’offsets](2026-11-16-un-ajout-au-dictionnaire-refuse-change-la-derniere-chaine.md) ; aucun changement furtif de sanityCheck dans ce correctif. À faire : relever appelants et invariant souhaité, témoin rouge réduit sur un état invalide construit proprement, puis contrôle correspondant. Ne pas deviner une comparaison de remplacement sans contrat, ni injecter un état invalide dans une base réelle. Fin : témoin rouge puis vert, invariant et appelants documentés, qualification pertinente. Aucun runtime lancé pour ce constat.
