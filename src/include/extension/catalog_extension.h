#pragma once

#include "catalog/catalog.h"

namespace rag3db {
namespace extension {

class RAG3DB_API CatalogExtension : public catalog::Catalog {
public:
    CatalogExtension() : Catalog() {}

    virtual void init() = 0;

    void invalidateCache();
};

} // namespace extension
} // namespace rag3db
