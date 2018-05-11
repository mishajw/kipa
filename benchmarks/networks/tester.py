import itertools
import logging
import random
from typing import List, Tuple

from benchmarks.networks import Network

log = logging.getLogger(__name__)


class SearchResult:
    def __init__(
            self,
            from_keys: List[str],
            to_keys: List[str],
            results: List[bool],
            message_ids: List[str]) -> None:
        self.from_keys = from_keys
        self.to_keys = to_keys
        self.results = results
        self.message_ids = message_ids

    @classmethod
    def empty(cls) -> "SearchResult":
        return cls([], [], [], [])

    def __len__(self):
        return len(self.message_ids)

    def __getitem__(self, index) -> Tuple[str, str, bool, str]:
        return \
            self.from_keys[index], \
            self.to_keys[index], \
            self.results[index], \
            self.message_ids[index]

    def add_result(
            self,
            from_key: str,
            to_key: str,
            result: bool,
            message_id: str) -> None:
        self.from_keys.append(from_key)
        self.to_keys.append(to_key)
        self.results.append(result)
        self.message_ids.append(message_id)

    def all_successes(self) -> bool:
        return all(self.results)

    def percentage_success(self) -> float:
        return sum(1 for r in self.results if r) / len(self.results)


def test_search(
        network: Network, from_key_id: str, to_key_id: str) -> Tuple[bool, str]:
    try:
        log.info(f"Testing search between {from_key_id} and {to_key_id}")
        output = network.exec_command(
            from_key_id,
            [
                "/root/kipa_cli",
                "search",
                "--key-id", to_key_id])

        success = "Search success" in output

        message_id = set([
            l["message_id"]
            for l in network.get_cli_logs(from_key_id)
            if "message_id" in l])
        assert len(message_id) == 1, \
            "Couldn't find exactly one `message_id` when testing search, " \
            f"found: {message_id}"
        message_id = next(iter(message_id))

        return success, message_id
    except AssertionError as e:
        log.error(
            "Error thrown when testing search "
            f"between {from_key_id} and {to_key_id}: {e}")
        return False, ""


def test_all_searches(network: Network) -> SearchResult:
    keys = network.get_all_keys()
    results = SearchResult.empty()
    for k1, k2 in itertools.permutations(keys, 2):
        success, message_id = test_search(network, k1, k2)
        results.add_result(k1, k2, success, message_id)
    return results


def sample_test_searches(
        network: Network, num_searches: int=None) -> SearchResult:
    if num_searches is None:
        num_searches = 500
    key_pairs = list(itertools.permutations(network.get_all_keys(), 2))
    num_searches = min(len(key_pairs), num_searches)
    results = SearchResult.empty()
    for k1, k2 in random.sample(key_pairs, num_searches):
        success, message_id = test_search(network, k1, k2)
        results.add_result(k1, k2, success, message_id)
    return results
