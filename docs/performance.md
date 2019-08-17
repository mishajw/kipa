# Performance of KIPA

*Required reading: [How graph search works.](./design.md#graph-search)*

The performance of a KIPA search depends on several factors:

1. The number of queries needed to find a node (discussed
   [here](#graph-performance)).
2. The network round-trip-time of a single KIPA query (discussed
   [here](#network-performance)).
3. The response time of a single KIPA query (discussed
   [here](#implementation-performance)).

<!-- TODO: Add TL;DR speeds to the factors, with overall calculation. -->

We discuss each of the factors in the following section.

## Factors

### Graph performance

We have several relevant parameters when discussing the number of queries:
- `D`: The number of dimensions used for key space.
- `N`: The number of nodes in the system.
- `E`: The number of neighbours (or edges) each node has.

We also have the width of the key space (i.e. the range of possible values in
each key space dimension). We set this to 1 to simplify equations.

We evaluate the graph performance as the expected number of queries required to
find a node. We first discuss how to calculate this when key space has 2
dimensions, then we generalize to N dimensions.

#### Estimating number of queries in 2 dimensions

Each search for a node has to travel some "distance" - the distance between the
start node and the end node. Each query they perform progresses the search by a
small distance.

##### Estimating search distance

We calculate the average distance each search must cross, denoted as `l`. The
distances are uniformly distributed from the minimum distance to the maximum
distance.  This minimum distance is 0, and the maximum distance is half the
width of the key space. It is half because the key space is wrapped. So, the
average distance each search must cross is quarter the width of key space:

<!-- TODO: is that correct? -->

```
l = (width of key space) / 4
l = 1/4
```

##### Estimating query distance

We calculate the average distance each query takes you towards your goal.

This depends on the *density* of the network, i.e. how many nodes exist in each
unit square of key space. As the entirety of key space is one unit square, the
density `d` of the key space is simple to calculate:

```
d = N / (width of key space)²
d = N / 1²
d = N
```

Next, we can calculate the average distance between a node and its neighbours.
We can think of this as the "radius" `r` of a node. Then, we need to find the
radius that would contain (according to the density of the key space) the
number of neighbours the node has:

```
area(r) * d = E
    area(r) = E/d
          r = area⁻¹(E/d)
          
applied to 2D:

area(r) * d = E
       πr²d = E
          r = √(E/πd)
```

Now we know how far away we can expect neighbours to be, but we don't know if
they'll be pointing in the same direction as the node we are looking for. To
account for this, we calculate the average angle between the node we're
searching for, and the neighbour closest to it (relative to the local node).

To do this, we use two pieces of information:
- The average angle between any two nodes is uniformly distributed in `[0,π]`.
- The average maximum of `n` numbers taken from `[0, 1]` is `n/(n+1)`.

So, the average angle `a` between a search node and the neighbour closest to it
is:

```
a = (1 - (E/(E+1))π
```

Finally, using the radius of a node `r`, and the average angle of neighbours
`a`, we can calculate the average distance each query takes us `l'`:

```
l' = cos(a)r
```

##### Estimating the number of queries

From the search distance `l` and the query distance `l'`, it is trivial to
calculate the average number of queries `q` needed for each search:

```
q = l / l'
```

#### Estimating step size in N dimensions

### Network performance

<!--
TODO: Read
http://www.caida.org/publications/papers/2004/tr-2004-02/tr-2004-02.pdf
-->

### Implementation performance

## Summary
