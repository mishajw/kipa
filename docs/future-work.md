# Future work

This document outlines some future work for KIPA.

## Adjusting key space for network density

One of the tunable parameters of KIPA is the number of dimensions used in key space. However, the
best value is dependent on how many nodes there are in KIPA. The intuition is that more dimensions
means more directions to get to other nodes - but if there aren't enough neighbours to *go* in all
directions, then more dimensions can degrade performance.

Would it be possible to infer the density of the network from the distance of your direct
neighbours? If so, nodes could gradually compare more and more dimensions as the network becomes
more dense. They could even compare partial dimensions, by e.g. scaling a dimension with 0.5. This
would mean that nodes with different inferred densities would still be speaking roughly the same
language.

## Using UDP for lower-latency requests

KIPA currently uses TCP for query requests. Due to handshakes to set up the connection, this can
cause higher latency. As we can send a lot of query requests per search, and some queries will have
to run sequentially, this can degrade search time.

Alternatively, KIPA could use UDP, which would remove the latency. This comes with two concerns:

**No reording out-of-order packets.** If we don't return large amounts of nodes, we can probably fit
query requests and responses in a single packet. So out-of-order requests shouldn't be a problem.

**No re-sending dropped packets.** KIPA can remedy this, not by resending packets manually, but by
sending *mutliple packets to different nodes*. This would mean that the reliability comes not from
a reliable connection to a single node, but many unreliable connections to many nodes.

## Improved privacy with noisy requests.

Currently, when searching for a node, KIPA will query nodes for the specific key its searching for.
This means that any queried nodes will be aware of what's being searched for. Alternatively, KIPA
could query using *the location in key space* rather than the key itself.

Additionally, we can add noise to the queried location. When searching far away from the node, the
noise won't matter: all we're expecting is to go in the correct direction. Once the search becomes
closer, less and less noise can be added.

This would further increase the privacy guarantees of KIPA searches.

## Using neighbour "garbage collection" to find better neighbours

Currently, a KIPA daemon periodically send an empty request to its neighbours in order to check they
are still alive. Instead, the daemon could query for its closest neighbours to itself: this would
allow the daemon to update its neighbours, while still checking the liveness of neighbours.
