import random
from itertools import permutations
from typing import NamedTuple, FrozenSet, Optional, Set

from graph_experiments import Node, Distance, TestArgs


class ConnectednessResults(NamedTuple):
    successful_percent: float
    mean_num_requests: float

    def __str__(self) -> str:
        return ",".join(
            [
                f"{self.successful_percent * 100:.2f}%",
                f"avg={self.mean_num_requests:.2f}",
            ]
        )


def test_nodes(
    nodes: FrozenSet[Node], distance: Distance, args: TestArgs
) -> "ConnectednessResults":
    assert args.num_graph_tests > 0
    results = [
        __run_test(nodes, distance, args) for _ in range(args.num_graph_tests)
    ]
    return ConnectednessResults(
        sum(r.successful_percent for r in results) / len(results),
        sum(r.mean_num_requests for r in results) / len(results),
    )


def __run_test(
    nodes: FrozenSet[Node], distance: Distance, args: TestArgs
) -> "ConnectednessResults":
    search_node_pairs = list(permutations(nodes, 2))
    search_node_pairs = random.sample(
        search_node_pairs, k=min(args.num_search_tests, len(search_node_pairs))
    )
    results = [
        __search(from_node, to_node, nodes, distance)
        for from_node, to_node in search_node_pairs
    ]
    results_success = list(filter(None, results))
    successful_percent = len(results_success) / len(results) if results else 0
    mean_num_requests = (
        sum(results_success) / len(results_success) if results_success else 0
    )
    return ConnectednessResults(successful_percent, mean_num_requests)


def __search(
    from_node: Node,
    to_node: Node,
    all_nodes: FrozenSet[Node],
    distance: Distance,
) -> Optional[int]:
    explored: Set[Node] = set()
    to_explore: Set[Node] = {from_node}
    while to_explore:
        exploring = min(
            to_explore,
            key=lambda n: distance.distance(to_node.key_space, n.key_space),
        )
        to_explore.remove(exploring)
        explored.add(exploring)

        if to_node.index in exploring.neighbours:
            return len(explored)
        new_nodes = frozenset(
            n for n in all_nodes if n.index in exploring.neighbours
        )
        to_explore.update(new_nodes.difference(explored))
    return None
