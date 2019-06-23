import itertools
import logging
import random
import time
from typing import List, NamedTuple

from simulation.backends import Backend
from simulation.networks import Network, NodeId, Node

log = logging.getLogger(__name__)


def sample_test_searches(
    network: Network, backend: Backend, num_searches: int
) -> "TestResult":
    log.info(f"Performing {num_searches} searches")
    nodes = filter(lambda n: not n.disconnect_before_tests, network.nodes)
    node_pairs = list(itertools.permutations(nodes, 2))
    if not node_pairs:
        return TestResult([], 0, 0, 0)
    random_node_pairs = [random.choice(node_pairs) for _ in range(num_searches)]
    results = []
    for i, (node1, node2) in enumerate(random_node_pairs):
        log.info(f"Performing search {i + 1}/{num_searches}")
        results.append(__test_search(backend, node1, node2))
    return TestResult.from_searches(results)


def __test_search(
    backend: Backend, from_node: Node, to_node: Node
) -> "SearchResult":
    try:
        log.info(f"Testing search between {from_node.id} and {to_node.id}")

        search_start_time = time.time()
        output = backend.run_command(
            from_node.id, ["search", "--key-id", to_node.key_id()]
        )
        search_end_time = time.time()
        search_time_sec = search_end_time - search_start_time

        success = output is not None and "Search unsuccessful" not in output

        message_id = set(
            [
                l["message_id"]
                for l in backend.get_cli_logs(from_node.id)
                if "message_id" in l
            ]
        )
        assert len(message_id) == 1, (
            "Couldn't find exactly one `message_id` when testing search, "
            f"found: {message_id}"
        )
        message_id = next(iter(message_id))

        num_requests = sum(
            1
            for l in backend.get_logs(from_node.id)
            if "message_id" in l
            and l["message_id"] == message_id
            and "making_request" in l
        )

        return SearchResult(
            from_node.id,
            to_node.id,
            success,
            message_id,
            num_requests,
            search_time_sec,
        )
    except AssertionError as e:
        log.error(
            "Error thrown when testing search "
            f"between {from_node.id} and {to_node.id}: {e}"
        )
        return SearchResult(from_node.id, to_node.id, False, "", 0, 0)


class TestResult(NamedTuple):
    search_results: List["SearchResult"]
    success_percentage: float
    average_num_requests: float
    average_search_times_sec: float

    @classmethod
    def from_searches(
        cls, search_results: List["SearchResult"]
    ) -> "TestResult":
        successful_results = list(filter(lambda r: r.success, search_results))
        if len(successful_results) == 0:
            return TestResult(search_results, 0, 0, 0)

        return TestResult(
            search_results,
            len(successful_results) / len(search_results),
            sum(map(lambda r: r.num_requests, successful_results))
            / len(successful_results),
            sum(map(lambda r: r.search_times_sec, successful_results))
            / len(successful_results),
        )


class SearchResult(NamedTuple):
    from_id: NodeId
    to_id: NodeId
    success: bool
    message_id: str
    num_requests: int
    search_times_sec: float
