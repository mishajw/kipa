import unittest

from simulation import networks, utils


class TestIpv6(unittest.TestCase):
    def test_all_searches(self):
        results = networks.configuration.Configuration(
            [networks.configuration.GroupConfiguration(5, ipv6=True)],
            connect_type=networks.configuration.ConnectType.ROOTED,
            num_connects=1).run(f"simulation_output/tests/"
                                f"test_ipv6_{utils.get_formatted_time()}")

        self.assertEqual(results["percentage_success"], 1)
