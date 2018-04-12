import unittest

from benchmarks import networks


class TestCyclicSearch(unittest.TestCase):
    def test_all_searches(self):
        network = networks.creator.create(3)
        networks.modifier.connect_nodes_cyclically(network)
        self.assertTrue(networks.tester.test_all_searches(network))


class TestRootedSearch(unittest.TestCase):
    def test_all_searches(self):
        network = networks.creator.create(3)
        [root_key_id] = network.get_random_keys(1)
        networks.modifier.connect_nodes_to_one(network, root_key_id)
        self.assertTrue(networks.tester.test_all_searches(network))
