import logging
import unittest

from simulation import networks, utils

log = logging.getLogger(__name__)


class TestCyclicSearch(unittest.TestCase):
    def test_all_searches(self):
        results = networks.configuration.Configuration(
            num_nodes=5,
            connect_type=networks.configuration.ConnectType.CYCLICAL,
            num_connects=1).run(f"simulation_output/tests/"
                                f"test_cyclic_{utils.get_formatted_time()}")

        self.assertEqual(results["percentage_success"], 1)


class TestRootedSearch(unittest.TestCase):
    def test_all_searches(self):
        results = networks.configuration.Configuration(
            num_nodes=5,
            connect_type=networks.configuration.ConnectType.ROOTED,
            num_connects=1).run(f"simulation_output/tests/"
                                f"test_rooted_{utils.get_formatted_time()}")

        self.assertEqual(results["percentage_success"], 1)
