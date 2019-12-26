import logging
import random

from matplotlib.path import Path

from simulation.benchmarks import SuccessSpeedBenchmark
from simulation.networks import Network

log = logging.getLogger(__name__)

NETWORK_SIZES = list(range(20, 201, 20))


class ScalabilityBenchmark(SuccessSpeedBenchmark):
    def __init__(self, output_directory: Path):
        super().__init__("scalability", NETWORK_SIZES, "Network size", output_directory)

    def get_network(self, network: Network, network_size: int) -> Network:
        assert len(network.nodes) >= max(
            NETWORK_SIZES
        ), "Configured network must be larger than max network size test."
        nodes = random.sample(network.nodes, network_size)
        return network.replace(nodes=nodes)
