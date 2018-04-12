import itertools
import logging

from benchmarks.networks import Network

log = logging.getLogger(__name__)


def test_search(network: Network, from_key_id: str, to_key_id: str) -> bool:
    log.info(f"Testing search between {from_key_id} and {to_key_id}")
    output = network.exec_command(
        from_key_id,
        [
            "/root/kipa_cli",
            "search",
            "--key-id", to_key_id])

    log.info(f"Search output:\n{output}")
    return "Search success" in output


def test_all_searches(network: Network):
    keys = network.get_all_keys()
    for k1, k2 in itertools.permutations(keys, 2):
        if not test_search(network, k1, k2):
            return False
    return True
