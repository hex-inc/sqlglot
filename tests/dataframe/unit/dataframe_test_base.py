import typing as t
import unittest

import hex.sqlglot as sqlglot
from hex.sqlglot import MappingSchema
from hex.sqlglot.dataframe.sql import SparkSession
from hex.sqlglot.dataframe.sql.dataframe import DataFrame
from hex.sqlglot.helper import ensure_list


class DataFrameTestBase(unittest.TestCase):
    def setUp(self) -> None:
        sqlglot.schema = MappingSchema()
        SparkSession._instance = None

    def compare_sql(
        self, df: DataFrame, expected_statements: t.Union[str, t.List[str]], pretty=False
    ):
        actual_sqls = df.sql(pretty=pretty)
        expected_statements = ensure_list(expected_statements)
        self.assertEqual(len(expected_statements), len(actual_sqls))
        for expected, actual in zip(expected_statements, actual_sqls):
            self.assertEqual(expected, actual)
