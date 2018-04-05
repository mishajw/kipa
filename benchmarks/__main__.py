"""Create, run, and manage collections of KIPA nodes in a KIPA network."""

from benchmarks.network_creator import create_network
import itertools
import logging

def main():
    """Test that each node can be search from every other node."""
    network = create_network(3)
    keys = network.get_all_keys()
    for k1, k2 in itertools.permutations(keys, 2):
        network.test_search(k1, k2)

if __name__ == "__main__":
    logging.basicConfig()
    logging.getLogger().setLevel(logging.DEBUG)
    logging.getLogger("docker").setLevel(logging.WARNING)
    logging.getLogger("urllib3").setLevel(logging.WARNING)
    main()

