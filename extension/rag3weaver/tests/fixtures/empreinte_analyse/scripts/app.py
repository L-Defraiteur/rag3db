from scripts.lib import Catalog, register
import os.path

DEFAULT = Catalog.create()
index = os.path.join('a', 'b'); s = index.upper(); line = s + 'x'


def helper(valeur):
    return [v for v in valeur]


register(Catalog)
for ordre in ("a", "b"):
    helper(ordre)
e2, t2 = helper("x"), helper("y")

if __name__ == "__main__":
    helper(DEFAULT)
