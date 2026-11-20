#include "lib.h"

namespace moteur {
Catalog Catalog::make() { return Catalog(); }
int Catalog::size() const { return 1; }

int demarrer() {
    auto total = Catalog::make();
    int count = 3;
    for (auto& item : {1, 2}) { count += item; }
    return total.size() + count;
}
}
