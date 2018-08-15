"""Create, run, and manage collections of KIPA nodes in a KIPA network."""

import argparse
import logging
import os

from simulation import networks, utils, comparisons, benchmarks


def main():
    parser = argparse.ArgumentParser("simulation")
    parser.add_argument(
        "-c",
        "--network_config",
        type=str,
        required=True,
        help="The file to read the simulation configuration from")
    parser.add_argument(
        "-o",
        "--output_directory",
        type=str,
        default="simulation_output",
        help="Where to output simulation results")
    parser.add_argument(
        "--comparison",
        type=str,
        choices=["angle"],
        default=None,
        help="Run a comparison of the performance on a variable")
    parser.add_argument(
        "--benchmark",
        type=str,
        choices=["reliability", "resilience"],
        default=None,
        help="Run a benchmark to see how well a configuration performs under "
             "varying conditions")

    args = parser.parse_args()
    network_config = args.network_config
    output_directory = args.output_directory

    if args.comparison is not None:
        if args.comparison == "angle":
            comparisons.run_angle_comparison(network_config, output_directory)
        else:
            raise ValueError(f"Unrecognized comparison type: {args.comparison}")
        return

    if args.benchmark is not None:
        if args.benchmark == "reliability":
            benchmarks.run_reliability_benchmark(
                network_config, output_directory)
        elif args.benchmark == "resilience":
            benchmarks.run_resilience_benchmark(
                network_config, output_directory)
        else:
            raise ValueError(f"Unrecognized benchmark type: {args.benchmark}")
        return

    configuration = networks.configuration.Configuration.from_yaml(
        network_config)
    configuration.run(os.path.join(
        args.output_directory, "configuration", utils.get_formatted_time()))


if __name__ == "__main__":
    logging.basicConfig()
    logging.getLogger().setLevel(logging.DEBUG)
    logging.getLogger("docker").setLevel(logging.WARNING)
    logging.getLogger("urllib3").setLevel(logging.WARNING)
    logging.getLogger("PIL").setLevel(logging.WARNING)
    main()
