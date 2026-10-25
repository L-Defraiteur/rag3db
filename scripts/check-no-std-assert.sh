#!/bin/sh
# Aucun <assert.h> ni <cassert> dans nos sources : l'assert du moteur est KU_ASSERT (common/assert.h).
# Le code vendu (third_party, vendor, dont le sous-module codeparsers et ses grammaires tree-sitter)
# n'est pas à nous : il est exclu.
! grep --color=auto -r --exclude-dir=third_party --exclude-dir=vendor --exclude-dir=node_modules "include <assert.h>\|include <cassert>" $1
