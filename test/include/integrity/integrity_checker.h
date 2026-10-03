#pragma once

// Le vérificateur d'intégrité du banc de concurrence (marche A1). Il contrôle des
// vérités sur une base après coup et rend la liste des violations, chacune avec la
// ligne fautive. Une liste vide veut dire « rien trouvé », pas « base saine » : il ne
// voit que ce qu'il cherche.
//
// Règle : ne jamais lire une propriété d'une extrémité de relation pour compter ou
// lister des relations. Lire a.id dans MATCH (a)-[r]->(b) joint la table des nœuds
// de a et efface en silence une relation dont a est supprimé, alors que count(r) la
// compte encore. La première version du vérificateur s'y est fait prendre (C2, étape
// 1) ; son témoin rouge est IntegrityCheckerWitness.
//
// Niveau 1 : par Cypher seulement (API publique), utilisable depuis n'importe quel
// processus. Niveau 2 : par les internes, voir checkLevel2.

#include <string>
#include <vector>

#include "main/connection.h"

namespace rag3db {
namespace testing {
namespace integrity {

struct Violation {
    std::string invariant;
    std::string detail;
};

// Les relations du banc portent les clés de leurs extrémités dans ces deux
// propriétés quand le schéma les déclare ; le vérificateur les compare alors aux
// clés des extrémités réelles.
constexpr const char* SOURCE_KEY_PROPERTY = "src_id";
constexpr const char* DESTINATION_KEY_PROPERTY = "dst_id";

std::vector<Violation> checkLevel1(main::Connection& connection);

// Niveau 2 : par les internes du stockage, dans une transaction de lecture ouverte sur
// la connexion. Il ne passe pas par le planificateur :
// - chaque ligne visible (lue par Cypher) se retrouve par l'index de clé primaire à son
//   propre offset (lookupPK) — un doublon visible y échoue forcément ;
// - le nombre de lignes visibles vu par le stockage (isVisibleNoLock sur tous les
//   offsets, supprimés compris) est celui que Cypher voit ;
// - la CSR, balayée dans chaque direction stockée depuis TOUS les offsets du côté lié,
//   supprimés compris, porte les mêmes relations dans les deux sens, et chaque
//   extrémité est une ligne visible.
// Clés primaires INT64 seulement (celles du banc) ; une autre est signalée, pas sautée.
std::vector<Violation> checkLevel2(main::Connection& connection);

// Toutes les lignes et toutes les relations, sans identifiant interne, triées : deux
// bases qui donnent les mêmes réponses ont le même vidage. Une relation pendante y
// figure avec « <missing> » à la place de la clé absente.
std::vector<std::string> canonicalDump(main::Connection& connection);

// Les lignes présentes dans un seul des deux vidages, avec leur côté.
std::vector<Violation> compareDumps(const std::vector<std::string>& before,
    const std::vector<std::string>& after, const std::string& beforeName,
    const std::string& afterName);

std::string describe(const std::vector<Violation>& violations);

} // namespace integrity
} // namespace testing
} // namespace rag3db
