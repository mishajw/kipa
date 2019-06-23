import logging
import os
import unittest

from simulation import networks, utils

log = logging.getLogger(__name__)


class TestCyclicSearch(unittest.TestCase):
    def test_all_searches(self):
        results = networks.configuration.Configuration(
            [networks.configuration.GroupConfiguration(5)],
            connect_type=networks.configuration.ConnectType.CYCLICAL,
            num_connects=1,
        ).run(
            os.path.join(
                "simulation_output/tests/cyclic",
                f"{utils.get_formatted_time()}",
            )
        )

        self.assertEqual(results["percentage_success"], 1)


class TestRootedSearch(unittest.TestCase):
    def test_all_searches(self):
        results = networks.configuration.Configuration(
            [networks.configuration.GroupConfiguration(5)],
            connect_type=networks.configuration.ConnectType.ROOTED,
            num_connects=1,
        ).run(
            os.path.join(
                "simulation_output/tests/rooted",
                f"{utils.get_formatted_time()}",
            )
        )

        self.assertEqual(results["percentage_success"], 1)
