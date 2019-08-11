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
from itertools import product

import matplotlib.pyplot as plt

from graph_experiments import (
    Args,
    TestStrategy,
    NeighbourStrategy,
    KeySpace,
    Node,
    Distance,
)
from graph_experiments.tester import ConnectednessResults, test_nodes


def main():
    parser = ArgumentParser("graph_experiments")
    parser.add_argument(
        "--neighbour-strategy", type=str, required=True, nargs="+"
    )
    parser.add_argument("--distance", type=str, required=True, nargs="+")
    parser.add_argument("--test-strategy", type=str, required=True, nargs="+")
    parser.add_argument("--num-nodes", type=int, default=[100], nargs="+")
    parser.add_argument(
        "--key-space-dimensions", type=int, default=[2], nargs="+"
    )
    parser.add_argument("--max-neighbours", type=int, default=[10], nargs="+")
    parser.add_argument("--num-search-tests", type=int, default=100)
    parser.add_argument("--num-graph-tests", type=int, default=1)
    parser.add_argument("--output-path", type=str, default="output.png")
    parser_args = parser.parse_args()

    all_strategy_names = list(
        product(
            parser_args.neighbour_strategy,
            parser_args.distance,
            parser_args.test_strategy,
        )
    )

    all_args = [
        Args(
            *args,
            num_search_tests=parser_args.num_search_tests,
            num_graph_tests=parser_args.num_graph_tests,
        )
        for args in product(
            parser_args.num_nodes,
            parser_args.key_space_dimensions,
            parser_args.max_neighbours,
        )
    ]

    for neighbour_name, distance_name, test_name in all_strategy_names:
        results = []
        for args in all_args:
            distance = Distance.get(distance_name, args)
            neighbour_strategy = NeighbourStrategy.get(
                neighbour_name, distance, args
            )
            test_strategy = TestStrategy.get(test_name)
            results.append(
                run(neighbour_strategy, distance, test_strategy, args)
            )
        plt.plot([r.mean_num_requests for r in results])

    plt.xticks(list(range(len(all_args))), all_args, rotation=45)
    plt.legend(all_strategy_names)
    plt.savefig(parser_args.output_path)
    plt.show()


def run(
    neighbour_strategy: NeighbourStrategy,
    distance: Distance,
    test_strategy: TestStrategy,
    args: Args,
) -> ConnectednessResults:
    nodes = frozenset(
        Node(i, KeySpace.random(args.key_space_dimensions))
        for i in range(args.num_nodes)
    )
    nodes = test_strategy.apply(nodes, neighbour_strategy)
    results = test_nodes(nodes, distance, args)
    print(
        type(neighbour_strategy).__name__,
        type(distance).__name__,
        type(test_strategy).__name__,
        args,
        results,
        sep="\t",
    )
    return results


if __name__ == "__main__":
    main()
