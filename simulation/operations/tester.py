import itertools
import logging
import random
from typing import List, NamedTuple

from simulation.backends import Backend
from simulation.backends.backend import CliCommand
from simulation.networks import Network, NodeId

log = logging.getLogger(__name__)


def sample_test_searches(network: Network, backend: Backend, num_searches: int) -> "TestResult":
    log.info(f"Performing {num_searches} searches")
    nodes = filter(lambda n: not n.disconnect_before_tests, network.nodes)
    node_pairs = list(itertools.permutations(nodes, 2))
    if not node_pairs:
        return TestResult([], 0, 0, 0)
    random_node_pairs = [random.choice(node_pairs) for _ in range(num_searches)]

    commands = [CliCommand(a.id, ["search", "--key-id", b.key_id()]) for a, b in random_node_pairs]
    command_results = backend.run_commands(commands)

    search_results: List[SearchResult] = []
    for (from_node, to_node), result in zip(random_node_pairs, command_results):
        success = result.successful() and "Search unsuccessful" not in result.stdout
        if result.cli_logs is None:
            search_results.append(SearchResult(from_node.id, to_node.id, False, "", 0, 0))
            continue

        message_id = set([l["message_id"] for l in result.cli_logs if "message_id" in l])
        assert (
            len(message_id) == 1
        ), "Couldn't find exactly one `message_id` when testing search, found: {message_id}"
        message_id = next(iter(message_id))

        num_requests = sum(
            1
            for l in backend.get_logs(from_node.id)
            if "message_id" in l and l["message_id"] == message_id and "making_request" in l
        )

        search_results.append(
            SearchResult(
                from_node.id, to_node.id, success, message_id, num_requests, result.duration_sec,
            )
        )

    return TestResult.from_searches(search_results)


class TestResult(NamedTuple):
    search_results: List["SearchResult"]
    success_percentage: float
    average_num_requests: float
    average_search_times_sec: float

    @classmethod
    def from_searches(cls, search_results: List["SearchResult"]) -> "TestResult":
        successful_results = list(filter(lambda r: r.success, search_results))
        if len(successful_results) == 0:
            return TestResult(search_results, 0, 0, 0)

        return TestResult(
            search_results,
            len(successful_results) / len(search_results),
            sum(map(lambda r: r.num_requests, successful_results)) / len(successful_results),
            sum(map(lambda r: r.search_times_sec, successful_results)) / len(successful_results),
        )


class SearchResult(NamedTuple):
    from_id: NodeId
    to_id: NodeId
    success: bool
    message_id: str
    num_requests: int
    search_times_sec: float
