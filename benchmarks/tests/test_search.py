import logging
import unittest

from benchmarks import networks

log = logging.getLogger(__name__)


class TestCyclicSearch(unittest.TestCase):
    def test_all_searches(self):
        results = networks.configuration.Configuration(
            num_nodes=5,
            connect_type=networks.configuration.ConnectType.CYCLICAL,
            num_connects=1).run("benchmarks_output/tests")

        self.assertTrue(results["percentage_success"] == 1)


class TestRootedSearch(unittest.TestCase):
    def test_all_searches(self):
        results = networks.configuration.Configuration(
            num_nodes=5,
            connect_type=networks.configuration.ConnectType.ROOTED,
            num_connects=1).run("benchmarks_output/tests")

        self.assertTrue(results["percentage_success"] == 1)
