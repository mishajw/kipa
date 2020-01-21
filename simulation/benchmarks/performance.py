import logging
from pathlib import Path

from simulation.benchmarks import SuccessSpeedBenchmark
from simulation.networks import Network, ConnectionQuality

log = logging.getLogger(__name__)

CONNECTION_QUALITIES = [x / 100 for x in range(0, 100, 10)]
"""
Network qualities ranging from 0% loss, 0ms delay, and 1mbps, to 90% loss, 1
second delay, and 10kbps
"""


class PerformanceBenchmark(SuccessSpeedBenchmark):
    def __init__(self, output_directory: Path):
        super().__init__(
            "performance", CONNECTION_QUALITIES, "Network quality (0-1 scale)", output_directory,
        )

    def get_network(self, network: Network, quality_rating: float) -> Network:
        quality = ConnectionQuality(
            loss=quality_rating, delay=quality_rating * 1000, rate=(1 - quality_rating) * 1000,
        )
        quality_network = network._replace(connection_quality=quality)

        # Set timeout high so that the daemon does not exit searches that take too long.
        return quality_network.map_nodes(
            lambda n: n.replace(daemon_args={**n.daemon_args, "search_timeout_sec": 10000})
        )
