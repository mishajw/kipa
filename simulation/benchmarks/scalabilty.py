import logging
import random
from typing import Iterator

from matplotlib.path import Path

from simulation.benchmarks import SuccessSpeedBenchmark
from simulation.networks import Network
from simulation.operations import simulator

log = logging.getLogger(__name__)

NETWORK_SIZES = list(range(20, 201, 20))


class ScalabilityBenchmark(SuccessSpeedBenchmark):
    def __init__(self, output_directory: Path):
        super().__init__(
            "scalability", NETWORK_SIZES, "Network size", output_directory
        )

    def get_results(self, network: Network) -> Iterator[dict]:
        for network_size in NETWORK_SIZES:
            nodes = [random.sample(network.nodes) for _ in range(network_size)]
            sized_network = network.replace(nodes=nodes)
            output_directory = self.output_directory / f"size_{network_size}"
            yield simulator.simulate(sized_network, output_directory)
