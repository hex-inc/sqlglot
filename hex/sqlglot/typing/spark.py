from __future__ import annotations

from hex.sqlglot import exp
from hex.sqlglot.typing.spark2 import EXPRESSION_METADATA

EXPRESSION_METADATA = {
    **EXPRESSION_METADATA,
    exp.CurrentTimezone: {"returns": exp.DataType.Type.VARCHAR},
    exp.Localtimestamp: {"returns": exp.DataType.Type.TIMESTAMPNTZ},
}
