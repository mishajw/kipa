import logging
import random
from pathlib import Path

from simulation.benchmarks import SuccessSpeedBenchmark
from simulation.networks import Network, Node

log = logging.getLogger(__name__)

REMOVE_FLAGS = {"neighbours_size", "neighbour_gc_enabled", "search_breadth"}
MALICIOUS_FEATURES = {"use-random-response", "use-protobuf", "use-tcp", "use-unix-socket"}

MALICIOUS_PROBABILITIES = [x / 10 for x in range(10)]


class ResilienceBenchmark(SuccessSpeedBenchmark):
    def __init__(self, output_directory: Path):
        super().__init__(
            "resilience", MALICIOUS_PROBABILITIES, "Malicious probability (%)", output_directory,
        )

    def get_network(self, network: Network, malicious_probability: float) -> Network:
        return network.map_nodes(
            lambda n: _to_malicious_node(n) if random.random() < malicious_probability else n
        )

    def format_parameter(self, parameter: float) -> str:
        return str(parameter * 100)


def _to_malicious_node(node: Node) -> Node:
    return node.replace(
        clear_default_features=True,
        additional_features=frozenset(MALICIOUS_FEATURES),
        daemon_args={k: v for k, v in node.daemon_args.items() if k not in REMOVE_FLAGS},
    )
