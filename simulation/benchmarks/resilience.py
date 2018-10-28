import copy
import logging
import math
import os
from typing import Iterator

from simulation import networks
from simulation.benchmarks import SuccessSpeedBenchmark

log = logging.getLogger(__name__)

MALICIOUS_PROBABILITIES = [x / 10 for x in range(10)]


class ResilienceBenchmark(SuccessSpeedBenchmark):
    def __init__(self, output_directory: str):
        super().__init__(
            "resilience",
            [p * 100 for p in MALICIOUS_PROBABILITIES],
            "Malicious probability (%)",
            output_directory)

    def get_results(self, network_config_path: str) -> Iterator[dict]:
        for malicious_probability in MALICIOUS_PROBABILITIES:
            configuration = networks.configuration.Configuration.from_yaml(
                network_config_path)

            # For each group, split into two: one that has the original
            # configuration, and one has malicious configuration
            split_groups = []
            for group in configuration.groups:
                # Calculate sizes for each group
                malicious_group_size = int(
                    math.floor(malicious_probability * group.size))
                group_size = group.size - malicious_group_size
                if malicious_group_size <= 0:
                    log.info(
                        f"Not splitting group of size {group_size} with "
                        f"malicious probability {malicious_probability}")
                    split_groups.append(group)
                    continue

                log.info(
                    f"Splitting group of size {group.size} into "
                    f"normal group of size {group_size}, "
                    f"and malicious group of size {malicious_group_size}, "
                    f"malicious probability is {malicious_probability}")

                # Copy the group and set the correct sizes
                malicious_group: networks.configuration.GroupConfiguration = \
                    copy.deepcopy(group)
                group.size = group_size
                malicious_group.size = malicious_group_size

                # Make the malicious group "malicious" by making it return
                # random responses to queries
                malicious_group.clear_default_features = True
                malicious_group.additional_features.extend(
                    ["use-random-response", "use-protobuf", "use-tcp",
                     "use-unix-socket"])
                malicious_group.test_searches = False
                malicious_group.daemon_args = {}

                split_groups.extend([group, malicious_group])

            # Update the configuration's groups
            configuration.groups = split_groups

            results = configuration.run(os.path.join(
                self.output_directory,
                f"prob_{malicious_probability}"))
            yield results["percentage_success"]
