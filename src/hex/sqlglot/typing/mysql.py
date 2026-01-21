from __future__ import annotations

from hex.sqlglot import exp
from hex.sqlglot.typing import EXPRESSION_METADATA

EXPRESSION_METADATA = {
    **EXPRESSION_METADATA,
    **{expr_type: {"returns": exp.DataType.Type.VARCHAR} for expr_type in (exp.Elt,)},
    exp.Localtime: {"returns": exp.DataType.Type.DATETIME},
}
