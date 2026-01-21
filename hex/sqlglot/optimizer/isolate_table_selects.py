from __future__ import annotations

import typing as t

from hex.sqlglot import alias, exp
from hex.sqlglot.errors import OptimizeError
from hex.sqlglot.optimizer.scope import traverse_scope
from hex.sqlglot.schema import ensure_schema

if t.TYPE_CHECKING:
    from hex.sqlglot._typing import E
    from hex.sqlglot.schema import Schema
    from hex.sqlglot.dialects.dialect import DialectType


def isolate_table_selects(
    expression: E,
    schema: t.Optional[t.Dict | Schema] = None,
    dialect: DialectType = None,
) -> E:
    schema = ensure_schema(schema, dialect=dialect)

    for scope in traverse_scope(expression):
        if len(scope.selected_sources) == 1:
            continue

        for _, source in scope.selected_sources.values():
            assert source.parent

            if (
                not isinstance(source, exp.Table)
                or not schema.column_names(source)
                or isinstance(source.parent, exp.Subquery)
                or isinstance(source.parent.parent, exp.Table)
            ):
                continue

            if not source.alias:
                raise OptimizeError("Tables require an alias. Run qualify_tables optimization.")

            source.replace(
                exp.select("*")
                .from_(
                    alias(source, source.alias_or_name, table=True),
                    copy=False,
                )
                .subquery(source.alias, copy=False)
            )

    return expression
