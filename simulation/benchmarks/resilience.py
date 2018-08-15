import logging
import math
import os
from typing import Iterator
import copy

from simulation import networks, utils
import matplotlib.pyplot as plt

log = logging.getLogger(__name__)

MALICIOUS_PROBABILITIES = [x / 10 for x in range(11)]


def run_resilience_benchmark(network_config_path: str, output_directory: str):
    output_directory = os.path.join(
        output_directory, "benchmarks/resilience", utils.get_formatted_time())
    if not os.path.isdir(output_directory):
        os.makedirs(output_directory)

    results = list(__get_results(network_config_path, output_directory))

    plt.title("Resilience benchmark")
    plt.xlabel("Malicious probability (%)")
    plt.ylabel("Search success (%)")
    plt.plot(
        [p * 100 for p in MALICIOUS_PROBABILITIES],
        [r * 100 for r in results])
    plt.savefig(os.path.join(output_directory, "results.png"))


def __get_results(
        network_config_path: str, output_directory: str) -> Iterator[float]:
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
                    f"Not splitting group of size {group_size} with malicious "
                    f"probability {malicious_probability}")
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

            # Make the malicious group "malicious" by making it return random
            # responses to queries
            malicious_group.clear_default_features = True
            malicious_group.additional_features.extend(
                ["use-random-response", "use-protobuf", "use-tcp",
                 "use-unix-socket"])
            malicious_group.test_searches = False

            split_groups.extend([group, malicious_group])

        # Update the configuration's groups
        configuration.groups = split_groups

        results = configuration.run(os.path.join(
            output_directory,
            f"prob_{malicious_probability}"))
        yield results["percentage_success"]
