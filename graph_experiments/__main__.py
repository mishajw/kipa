"""
Benchmarking set up for testing neighbour selection algorithms for use in KIPA.

This is a simplified set up of the code in src/graph. This allows for quick
prototyping and testing of new algorithms in a perfect environment.

To try a new neighbour selection algorithm, implement the `NeighbourStrategy`
interface and choose `TestStrategy` to test it against.

For example, to benchmark the "random" and  "closest" neighbour strategies
against the "all-knowing" test strategy, against different numbers of nodes,
run:

  python -m graph_experiments \
    --num-nodes 10 20 30 40 50
    --neighbour-strategy random closest \
    --test-strategy all-knowing
"""

from argparse import ArgumentParser

import matplotlib.pyplot as plt

from graph_experiments import (
    Args,
    TestStrategy,
    NeighbourStrategy,
    KeySpace,
    Node,
)
from graph_experiments.tester import ConnectednessResults, test_nodes


def main():
    parser = ArgumentParser("graph_experiments")
    parser.add_argument(
        "--neighbour-strategy", type=str, required=True, nargs="+"
    )
    parser.add_argument("--test-strategy", type=str, required=True)
    parser.add_argument("--num-nodes", type=int, default=[100], nargs="+")
    parser.add_argument(
        "--key-space-dimensions", type=int, default=[2], nargs="+"
    )
    parser.add_argument("--max-neighbours", type=int, default=[10], nargs="+")
    parser.add_argument("--output-path", type=str, default="output.png")
    parser_args = parser.parse_args()
    all_args = Args.create(parser_args)

    test_strategy = TestStrategy.get(parser_args.test_strategy)
    for neighbour_strategy_name in parser_args.neighbour_strategy:
        neighbour_strategy = NeighbourStrategy.get(neighbour_strategy_name)
        results = [
            run(neighbour_strategy, test_strategy, arg).mean_num_requests
            for arg in all_args
        ]
        plt.plot(results)
    plt.xticks(list(range(len(all_args))), all_args, rotation=45)
    plt.legend(parser_args.neighbour_strategy)
    plt.show()
    plt.savefig(parser_args.output_path)


def run(
    neighbour_strategy: "NeighbourStrategy",
    test_strategy: "TestStrategy",
    args: "Args",
) -> "ConnectednessResults":
    nodes = frozenset(
        Node(i, KeySpace.random(args.key_space_dimensions))
        for i in range(args.num_nodes)
    )
    nodes = test_strategy.connect_nodes(nodes, neighbour_strategy, args)
    results = test_nodes(nodes)
    print(type(neighbour_strategy).__name__, args, results, sep="\t")
    return results


if __name__ == "__main__":
    main()
