#pragma once

namespace moteur {
class Table;

class Catalog {
public:
    static Catalog make();
    int size() const;
};
}
