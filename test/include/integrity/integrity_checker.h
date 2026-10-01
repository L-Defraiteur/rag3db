#pragma once

// Le vérificateur d'intégrité du banc de concurrence (marche A1). Il contrôle des
// vérités sur une base après coup et rend la liste des violations, chacune avec la
// ligne fautive. Une liste vide veut dire « rien trouvé », pas « base saine » : il ne
// voit que ce qu'il cherche.
//
// Niveau 1 : par Cypher seulement (API publique), utilisable depuis n'importe quel
// processus. Le niveau 2 (internes : index de clé primaire, CSR) vient à l'étape 2.

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

std::string describe(const std::vector<Violation>& violations);

} // namespace integrity
} // namespace testing
} // namespace rag3db
