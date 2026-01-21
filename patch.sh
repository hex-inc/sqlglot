#!/bin/bash
set -euo pipefail
set -o xtrace

# this will fail if you don't have a tag checked out
VERSION=$(git describe --exact-match --tags)

# update all imports to hex.sqlglot
find sqlglot tests -type f -name \*.py -exec \
    sed -i '' -e 's/import sqlglot$/import hex.sqlglot as sqlglot/g' \
              -e 's/import sqlglot./import hex.sqlglot./g' \
              -e 's/from sqlglot/from hex.sqlglot/g' {} ';'
git add sqlglot tests

# move into hex subdirectory
rm -rf src && mkdir -p src/hex
git mv sqlglot src/hex

# patch pyproject.toml
sed -i '' \
    -e 's/^name = "sqlglot"$/name = "hex-sqlglot"/' \
    -e 's|version_file = "sqlglot/_version.py"|version_file = "src/hex/sqlglot/_version.py"|' \
    -e 's/include = \["sqlglot", "sqlglot\.\*"\]/where = ["src"]\nnamespaces = true/' \
    pyproject.toml

git add pyproject.toml

# commit, tag, & push
git commit --message "hex.sqlglot namespace package for $VERSION"
git tag "$VERSION.post"
git push origin tag "$VERSION.post"
