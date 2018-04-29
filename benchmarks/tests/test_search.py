import logging
import unittest

from benchmarks import networks

log = logging.getLogger(__name__)


class TestCyclicSearch(unittest.TestCase):
    def test_all_searches(self):
        network = networks.creator.create(3)
        networks.modifier.ensure_alive(network)
        networks.modifier.connect_nodes_cyclically(network)

        results = networks.tester.test_all_searches(network)
        log.info(results.percentage_success())
        self.assertTrue(results.all_successes())


class TestRootedSearch(unittest.TestCase):
    def test_all_searches(self):
        network = networks.creator.create(32)
        networks.modifier.ensure_alive(network)
        [root_key_id] = network.get_random_keys(1)
        networks.modifier.connect_nodes_to_one(network, root_key_id)

        results = networks.tester.test_all_searches(network)
        log.info(results.percentage_success())
        self.assertTrue(results.all_successes())
