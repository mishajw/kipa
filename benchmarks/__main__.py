"""Create, run, and manage collections of KIPA nodes in a KIPA network."""

import logging

from benchmarks import networks


def main():
    """Test that each node can be search from every other node."""
    network = networks.creator.create(2)
    assert networks.tester.test_all_searches(network)


if __name__ == "__main__":
    logging.basicConfig()
    logging.getLogger().setLevel(logging.DEBUG)
    logging.getLogger("docker").setLevel(logging.WARNING)
    logging.getLogger("urllib3").setLevel(logging.WARNING)
    main()
