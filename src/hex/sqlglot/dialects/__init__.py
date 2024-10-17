# ruff: noqa: F401
"""
## Dialects

While there is a SQL standard, most SQL engines support a variation of that standard. This makes it difficult
to write portable SQL code. SQLGlot bridges all the different variations, called "dialects", with an extensible
SQL transpilation framework.

The base `sqlglot.dialects.dialect.Dialect` class implements a generic dialect that aims to be as universal as possible.

Each SQL variation has its own `Dialect` subclass, extending the corresponding `Tokenizer`, `Parser` and `Generator`
classes as needed.

### Implementing a custom Dialect

Creating a new SQL dialect may seem complicated at first, but it is actually quite simple in SQLGlot:

```python
from hex.sqlglot import exp
from hex.sqlglot.dialects.dialect import Dialect
from hex.sqlglot.generator import Generator
from hex.sqlglot.tokens import Tokenizer, TokenType


class Custom(Dialect):
    class Tokenizer(Tokenizer):
        QUOTES = ["'", '"']  # Strings can be delimited by either single or double quotes
        IDENTIFIERS = ["`"]  # Identifiers can be delimited by backticks

        # Associates certain meaningful words with tokens that capture their intent
        KEYWORDS = {
            **Tokenizer.KEYWORDS,
            "INT64": TokenType.BIGINT,
            "FLOAT64": TokenType.DOUBLE,
        }

    class Generator(Generator):
        # Specifies how AST nodes, i.e. subclasses of exp.Expression, should be converted into SQL
        TRANSFORMS = {
            exp.Array: lambda self, e: f"[{self.expressions(e)}]",
        }

        # Specifies how AST nodes representing data types should be converted into SQL
        TYPE_MAPPING = {
            exp.DataType.Type.TINYINT: "INT64",
            exp.DataType.Type.SMALLINT: "INT64",
            exp.DataType.Type.INT: "INT64",
            exp.DataType.Type.BIGINT: "INT64",
            exp.DataType.Type.DECIMAL: "NUMERIC",
            exp.DataType.Type.FLOAT: "FLOAT64",
            exp.DataType.Type.DOUBLE: "FLOAT64",
            exp.DataType.Type.BOOLEAN: "BOOL",
            exp.DataType.Type.TEXT: "STRING",
        }
```

The above example demonstrates how certain parts of the base `Dialect` class can be overridden to match a different
specification. Even though it is a fairly realistic starting point, we strongly encourage the reader to study existing
dialect implementations in order to understand how their various components can be modified, depending on the use-case.

----
"""

from hex.sqlglot.dialects.athena import Athena
from hex.sqlglot.dialects.bigquery import BigQuery
from hex.sqlglot.dialects.clickhouse import ClickHouse
from hex.sqlglot.dialects.databricks import Databricks
from hex.sqlglot.dialects.dialect import Dialect, Dialects
from hex.sqlglot.dialects.doris import Doris
from hex.sqlglot.dialects.drill import Drill
from hex.sqlglot.dialects.duckdb import DuckDB
from hex.sqlglot.dialects.hive import Hive
from hex.sqlglot.dialects.mysql import MySQL
from hex.sqlglot.dialects.oracle import Oracle
from hex.sqlglot.dialects.postgres import Postgres
from hex.sqlglot.dialects.presto import Presto
from hex.sqlglot.dialects.prql import PRQL
from hex.sqlglot.dialects.redshift import Redshift
from hex.sqlglot.dialects.snowflake import Snowflake
from hex.sqlglot.dialects.spark import Spark
from hex.sqlglot.dialects.spark2 import Spark2
from hex.sqlglot.dialects.sqlite import SQLite
from hex.sqlglot.dialects.starrocks import StarRocks
from hex.sqlglot.dialects.tableau import Tableau
from hex.sqlglot.dialects.teradata import Teradata
from hex.sqlglot.dialects.trino import Trino
from hex.sqlglot.dialects.tsql import TSQL
