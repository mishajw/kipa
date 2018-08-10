# Benchmarks

Benchmarking is performed on KIPA nodes in Docker containers, in order to track
aspects such as reliability and performance. This document discusses the results
of the benchmarks, but also shows how to run the benchmarks yourself.

## Running benchmarks

Benchmarking exists in the `./simulation` project. In order to run the
benchmarks, you can execute:

```bash
python -m simulation \
  --benchmark $BENCHMARK_TYPE \
  --network_config $CONFIGURATION
```

The `$BENCHMARK_TYPE` can be one of the types mentioned in this document:
`reliability`, `resilience`, and `speed`. The `$CONFIGURATION` is YAML file
describing the network (examples are given in `./resources/simulaton_configs`).

Prerequisites from the [README.md](../README.md#simulation) are required to run
benchmarks.

## Benchmark types and results

### Reliability

This benchmark evaluates how many searches are successful as increasing amounts
of nodes become unresponsive. After nodes are connected to each other, a
percentage of them are removed from the network, then the searches are
performed. The purpose is to show that KIPA does not have any single point of
failure, and is therefore a decentralised system.

#### Results
TODO

### Resilience

This benchmark evaluates how robust networks are to "malicious" nodes.
"Malicious", in the context of this benchmark, means that the node responds to
requests with false information (i.e. returning fake results with non-existent
IP addresses and keys). The purpose is to show that KIPA is resilient against
bad actors joining the system.

#### Results
TODO

### Speed

This benchmark evaluates the "speed" of the network as bandwidth, latency, and
packet loss of the network deteriorates. "Speed" is determined by the average
time to complete a search request (we also look at the variation in times). The
purpose is to show that KIPA is fast enough to be used as an IP address
resolution system.

#### Results
TODO

