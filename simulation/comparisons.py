import logging
import os

from simulation import networks, utils

log = logging.getLogger(__name__)


def run_angle_comparison(
        network_config_path: str, output_directory: str) -> None:
    output_directory = os.path.join(
        output_directory, "comparison", "angle", utils.get_formatted_time())

    angle_weighting_values = [x / 100 for x in range(0, 100, 10)]

    best_percentage_success = 0.0
    best_angle_weighting = 0.0

    for angle_weighting in angle_weighting_values:
        log.info(f"Trying angle weighting {angle_weighting}")

        configuration = networks.configuration.Configuration.from_yaml(
            network_config_path)

        for group in configuration.groups:
            group.daemon_args["angle_weighting"] = angle_weighting
            group.daemon_args["distance_weighting"] = 1 - angle_weighting

        configuration_output_directory = os.path.join(
            output_directory, f"aw-{angle_weighting:.3f}")

        results = configuration.run(configuration_output_directory)
        percentage_success = results["percentage_success"]

        log.info(f"Got {percentage_success * 100}% success "
                 f"for angle weighting {angle_weighting}")

        if percentage_success > best_percentage_success:
            best_percentage_success = percentage_success
            best_angle_weighting = angle_weighting

    log.info(f"Found best angle weighting {best_angle_weighting} "
             f"with {best_percentage_success * 100}% success")
