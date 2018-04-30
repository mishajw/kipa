import itertools
import logging
import random
from typing import List

from benchmarks.networks import Network

log = logging.getLogger(__name__)


class SearchResult:
    def __init__(
            self,
            from_keys: List[str],
            to_keys: List[str],
            results: List[bool]) -> None:
        self.from_keys = from_keys
        self.to_keys = to_keys
        self.results = results

    @classmethod
    def empty(cls) -> "SearchResult":
        return cls([], [], [])

    def add_result(self, from_key: str, to_key: str, result: bool) -> None:
        self.from_keys.append(from_key)
        self.to_keys.append(to_key)
        self.results.append(result)

    def all_successes(self) -> bool:
        return all(self.results)

    def percentage_success(self) -> float:
        return sum(1 for r in self.results if r) / len(self.results)


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


def test_all_searches(network: Network) -> SearchResult:
    keys = network.get_all_keys()
    results = SearchResult.empty()
    for k1, k2 in itertools.permutations(keys, 2):
        results.add_result(k1, k2, test_search(network, k1, k2))
    return results


def sample_test_searches(
        network: Network, num_searches: int=500) -> SearchResult:
    key_pairs = list(itertools.permutations(network.get_all_keys(), 2))
    num_searches = min(len(key_pairs), num_searches)
    results = SearchResult.empty()
    for k1, k2 in random.sample(key_pairs, num_searches):
        results.add_result(k1, k2, test_search(network, k1, k2))
    return results

