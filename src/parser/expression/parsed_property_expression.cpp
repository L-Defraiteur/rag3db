#include "parser/expression/parsed_property_expression.h"

#include "common/serializer/deserializer.h"

using namespace rag3db::common;

namespace rag3db {
namespace parser {

std::unique_ptr<ParsedPropertyExpression> ParsedPropertyExpression::deserialize(
    Deserializer& deserializer) {
    std::string propertyName;
    deserializer.deserializeValue(propertyName);
    return std::make_unique<ParsedPropertyExpression>(std::move(propertyName));
}

} // namespace parser
} // namespace rag3db
