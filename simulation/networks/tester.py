import itertools
import logging
import random
import time
from typing import List, Tuple

from simulation.networks import Network

log = logging.getLogger(__name__)


class SearchResult:
    def __init__(
            self,
            from_keys: List[str],
            to_keys: List[str],
            results: List[bool],
            message_ids: List[str],
            num_requests: List[int],
            search_times_sec: List[float]) -> None:
        self.from_keys = from_keys
        self.to_keys = to_keys
        self.results = results
        self.message_ids = message_ids
        self.num_requests = num_requests
        self.search_times_sec = search_times_sec

    @classmethod
    def empty(cls) -> "SearchResult":
        return cls([], [], [], [], [], [])

    def __len__(self):
        return len(self.message_ids)

    def __getitem__(self, index) -> Tuple[str, str, bool, str, int, float]:
        return \
            self.from_keys[index], \
            self.to_keys[index], \
            self.results[index], \
            self.message_ids[index], \
            self.num_requests[index], \
            self.search_times_sec[index]

    def add_result(
            self,
            from_key: str,
            to_key: str,
            result: bool,
            message_id: str,
            num_requests: int,
            search_time_sec: float) -> None:
        self.from_keys.append(from_key)
        self.to_keys.append(to_key)
        self.results.append(result)
        self.message_ids.append(message_id)
        self.num_requests.append(num_requests)
        self.search_times_sec.append(search_time_sec)

    def all_successes(self) -> bool:
        return all(self.results)

    def percentage_success(self) -> float:
        if len(self.results) == 0:
            return 0.0
        return sum(1 for r in self.results if r) / len(self.results)

    def average_num_requests(self) -> float:
        successful_num_requests = [
            nr for nr, r in zip(self.num_requests, self.results) if r]
        if len(successful_num_requests) == 0:
            return 0.0
        return sum(successful_num_requests) / len(successful_num_requests)

    def average_search_time_sec(self) -> float:
        successful_search_times_sec = [
            t for t, r in zip(self.search_times_sec, self.results) if r]
        if not successful_search_times_sec:
            return 0.0
        return sum(successful_search_times_sec) / \
               len(successful_search_times_sec)


def test_search(
        network: Network,
        from_key_id: str,
        to_key_id: str) -> Tuple[bool, str, int, float]:
    try:
        log.info(f"Testing search between {from_key_id} and {to_key_id}")

        search_start_time = time.time()
        output = network.exec_command(
            from_key_id,
            [
                "/root/kipa_cli",
                "search",
                "--key-id", to_key_id])
        search_end_time = time.time()
        search_time_sec = search_end_time - search_start_time

        success = "Search success" in output

        message_id = set([
            l["message_id"]
            for l in network.get_cli_logs(from_key_id)
            if "message_id" in l])
        assert len(message_id) == 1, \
            "Couldn't find exactly one `message_id` when testing search, " \
            f"found: {message_id}"
        message_id = next(iter(message_id))

        num_requests = sum(
            1
            for l in network.get_logs(from_key_id)
            if "message_id" in l
            and l["message_id"] == message_id
            and "making_request" in l)

        return success, message_id, num_requests, search_time_sec
    except AssertionError as e:
        log.error(
            "Error thrown when testing search "
            f"between {from_key_id} and {to_key_id}: {e}")
        return False, "", 0, 0


def test_all_searches(network: Network) -> SearchResult:
    keys = network.get_all_keys()
    results = SearchResult.empty()
    for k1, k2 in itertools.permutations(keys, 2):
        success, message_id, num_requests, search_time_sec = \
            test_search(network, k1, k2)
        results.add_result(
            k1, k2, success, message_id, num_requests, search_time_sec)
    return results


def sample_test_searches(
        network: Network, num_searches: int = None) -> SearchResult:
    if num_searches is None:
        num_searches = 500
    key_pairs = list(itertools.permutations(network.get_search_keys(), 2))
    results = SearchResult.empty()
    for k1, k2 in [random.choice(key_pairs) for _ in range(num_searches)]:
        success, message_id, num_requests, search_time_sec = \
            test_search(network, k1, k2)
        results.add_result(
            k1, k2, success, message_id, num_requests, search_time_sec)
    return results
