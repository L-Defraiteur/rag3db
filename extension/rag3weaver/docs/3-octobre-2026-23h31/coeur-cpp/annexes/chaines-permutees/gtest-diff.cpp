// Un EXPECT_EQ rouge entre deux chaînes de N lignes : ce que gtest réserve pour en écrire la
// différence.
#include <string>
#include "gtest/gtest.h"
TEST(GtestDiff, TwoLongMultiLineStrings) {
    const int n = std::stoi(std::getenv("LINES"));
    std::string a, b;
    for (int i = 0; i < n; i++) {
        a += std::to_string(i) + "|[vieux,vieux]\n";
        b += std::to_string(i) + (i % 500 == 0 ? "|[fichier,vieux]\n" : "|[vieux,vieux]\n");
    }
    EXPECT_EQ(a, b);
}
