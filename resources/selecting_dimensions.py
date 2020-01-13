#!/usr/bin/env python

# This script helps select the number of dimensions to use for KIPA's key space.
#
# Let's outline the problem:
# - We have an N-dimensional euclidean space.
# - The space contains a fixed number of nodes.
# - Each node is connected to the K nodes closest to it.
# - The goal is to minimize *the average number of steps between any two nodes*.
# - We want to optimize the number of dimensions in order to aid this goal.
#
# For example, we might want to get from S to E:
#
#     +--------------------+
#     |           x      x |
#     |                    |
#     |   x     x      E   |
#     |                    |
#     |         x          |
#     |  x          x      |
#     |                  x |
#     |    S    x          |
#     |                    |
#     |        x      x    |
#     +--------------------+

from scipy.special import betainc
from scipy.special import gamma
from typing import Callable
from typing import List
import math
import numpy as np

np.set_printoptions(suppress=True)
SPACE_WIDTH = 1


# The first step in solving this problem, is to find a way to calculate the expected number of steps
# between nodes, given the number of dimensions (`dims`), the number of nodes (`nodes`) and the
# number of edges on each node (`edges`).
def calc_num_steps(dims: int, nodes: int, edges: int) -> float:
    # First, we define some helper functions for N-dimensional sphere maths. Some of the functions
    # we use are difficult to calculate the inverse of, as they use gamma and beta functions. As I'm
    # not great at maths, we approximate the inverse.
    # Equations from: http://docsdrive.com/pdfs/ansinet/ajms/2011/66-70.pdf
    calc_sphere_volume = lambda r: (math.pi ** (dims / 2)) / gamma((dims / 2) + 1) * (r ** dims)
    calc_sphere_radius = approx_inverse(calc_sphere_volume, list(np.linspace(0, 100, 1000)))
    calc_sphere_filled = lambda a: 1 - (0.5 * betainc((dims + 1) / 2, 0.5, math.sin(a) ** 2))
    calc_sphere_angle = approx_inverse(calc_sphere_filled, list(np.linspace(0, math.pi / 2, 10)))

    # In order to do this, we stop thinking about the graph, and instead think about a single node
    # and it's direct neighbours.
    #
    # Each node has a set of neighbours, typically surrounding the node like so:
    #     +-------------------+
    #     |                   |
    #     |        1          |
    #     |               2   |
    #     |                   |
    #     |      3  x         |
    #     |                   |
    #     |            4      |
    #     |    5              |
    #     |                   |
    #     +-------------------+
    #
    # We can conceptualize the node and its neighbours as a sphere, where the node is in the centre
    # and the radius of the sphere is the distance to the furthest node:
    #     +-------------------+
    #     |                   |
    #     |       .....       |
    #     |     ..     ..     |
    #     |    .         .    |
    #     |   .     x     .   |
    #     |    .         .    |
    #     |     ..     ..     |
    #     |       .....       |
    #     |                   |
    #     +-------------------+
    #
    # How can we estimate the radius of this sphere? Well, we can calculate the average number of
    # nodes per square unit, and we also know how many nodes should be in this sphere: the number of
    # neighbours this node has! From this, we can figure out the radius:
    space_volume = SPACE_WIDTH ** dims
    nodes_per_unit_cube = nodes / space_volume
    sphere_volume = edges / nodes_per_unit_cube
    sphere_radius = calc_sphere_radius(sphere_volume)

    # So, now we know the radius, we know the distance to the furthest neighbour. But, when we're
    # moving across the space, our furthest neighbour might not be in the right direction!
    #
    # Instead we need to ask: given a random direction, what can we expect is the furthest neighbour
    # we have in that direction?
    #
    # For example, even though neighbour 5 might be the furthest neighbour, neighbour 4 is closest
    # to the goal `G`:
    #     +-------------------+
    #     |                   |
    #     |        1          |
    #     |               2   |
    #     |                   |
    #     |      3  x        5|
    #     |                   |
    #     |          4        |
    #     |                   |
    #     |         G         |
    #     +-------------------+
    #
    # We want to calculate the expected distance between `x` and neighbour 4.
    #
    # We can figure this out using a few pieces of information:
    # - All of our neighbours are *uniformly distributed* within the sphere.
    # - Given a list of N random numbers from 0-1, the expected largest number is `N/(N+1)`.
    #
    # This means that we can expect our furthest neighbour to be at the edge of the "+" part of the
    # sphere, where the "+" takes up `N/(N+1)`% of the *volume* of the sphere:
    #     +-------------------+
    #     |                   |
    #     |       .....       |
    #     |     ..+++++..     |
    #     |    .+++++++++.    |
    #     |   .+++++x+++++.   |
    #     |    .+++++++++.    |
    #     |     ..     ..     |
    #     |       .....       |
    #     |         G         |
    #     +-------------------+
    #
    # So, we need to figure out how far away edge of the "+" part is from the node. We do this by
    # first calculating how much of the sphere should be filled with "+".
    sphere_filled = edges / (edges + 1)

    # We then figure out the angle between the sphere's centre and the edge of the sphere where the
    # "+" stops:
    #     +-------------------+
    #     |.++++++++S++++++++.|
    #     |.+++++++/+\+++++++.|
    #     |.++++++/+++\++++++.|
    #     | .++++/+++++\++++. |
    #     |  .++/+++++++\++.  |
    #     |   ..         ..   |
    #     |     .........     |
    #     |         G         |
    #     +-------------------+
    sphere_angle = calc_sphere_angle(sphere_filled)

    # And then calculating the distance to the centre of the edge of the "+" part:
    #     +-------------------+
    #     |.++++++++S++++++++.|
    #     |.++++++++|++++++++.|
    #     |.++++++++|++++++++.|
    #     | .+++++++|+++++++. |
    #     |  .++++++|++++++.  |
    #     |   ..         ..   |
    #     |     .........     |
    #     |         G         |
    #     +-------------------+
    #
    # This is the expected *distance to the neighbour that is closest to the goal*. In other words,
    # at each step of the graph search, this is how far we can expect to move:
    step_size = sphere_radius * math.cos(sphere_angle / 2)

    # We can then figure out the average distance we'll need to cover when travelling between two
    # nodes, using an approximation.
    # Source: https://math.stackexchange.com/questions/1976842
    distance = math.sqrt(dims / 6 - 7 / 120) * SPACE_WIDTH

    # And finally we can calculate the expected number of steps needed to cover this distance!
    num_steps = distance / step_size
    return num_steps


# Now to use our calculations: for a given number of nodes and edges, we find the number of
# dimensions that result in the lowest step size:
def calc_best_dims(nodes: int, edges: int) -> float:
    calc_num_steps_for_dims = lambda dims: calc_num_steps(dims=dims, nodes=nodes, edges=edges)
    # Bit hacky, but we can reuse approx_inverse to find the `dims` that gives the result closest to
    # zero.
    best_dims = approx_inverse(calc_num_steps_for_dims, range(2, 20))(0)
    return best_dims


def approx_inverse(f: Callable[[float], float], inputs: List[float]) -> Callable[[float], float]:
    outputs = np.vectorize(f)(inputs)

    def f_inverse(output_expected):
        closest_idx = np.argmin(abs(outputs - output_expected))
        return inputs[closest_idx]

    return f_inverse


# Finally, let's print out the best number of dimensions for 10-1000 nodes...
print("nodes", "dims", sep="\t")
for nodes in range(10, 1000, 10):
    print(nodes, calc_best_dims(nodes, edges=10), sep="\t")
# And it looks like 19 becomes the best number of dimensions once we hit ~80 nodes!

# TODO: Read this paper on average network latency across the globe, in order to translate "number
# of steps" to an amount of time:
# http://www.caida.org/publications/papers/2004/tr-2004-02/tr-2004-02.pdf
