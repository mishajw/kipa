import unittest
from pathlib import Path

from simulation import utils
from simulation.key_creator import KeyCreator
from simulation.networks import Network
from simulation.operations import simulator


class TestIpv6(unittest.TestCase):
    def test_all_searches(self):
        network = Network.from_config(
            {"num_search_tests": 10, "ipv6": True, "groups": [{"size": 10}],}, KeyCreator(),
        )
        path = Path("simulation_output/tests/ipv6") / f"{utils.get_formatted_time()}"

        result = simulator.simulate(network, path)
        self.assertEqual(result.success_percentage, 1)
