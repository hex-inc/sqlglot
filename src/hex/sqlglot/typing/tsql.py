from __future__ import annotations

from hex.sqlglot import exp
from hex.sqlglot.typing import EXPRESSION_METADATA

EXPRESSION_METADATA = {
    **EXPRESSION_METADATA,
    exp.Radians: {"annotator": lambda self, e: self._annotate_by_args(e, "this")},
}
