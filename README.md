# KIPA: Key to IP Address

[![Crates.io](https://img.shields.io/crates/v/kipa.svg)](https://crates.io/crates/kipa)
[![Build Status](https://drone.spritsail.io/api/badges/mishajw/kipa/status.svg)](https://drone.spritsail.io/mishajw/kipa)
[![Documentation](https://docs.rs/kipa/badge.svg)](https://docs.rs/kipa/)

A distributed Key to IP Address query network.

## What is KIPA?

KIPA is a look-up service for finding out which IP addresses belong to a public
key. Each node on the KIPA network allows itself to be looked up by its key, and
is used for looking up other nodes in the network.

It is **distributed**, meaning that there is no single server on which the
network relies (backed up by [benchmarks](./docs/benchmarks.md#reliability)).

It is **zero-trust**, meaning that even with malicious nodes in the network, the
network is still secure and reliable (backed up by
[benchmarks](./docs/benchmarks.md#resilience)).

It is **fast**, with look-ups taking (TODO add figures) (backed up by
[benchmarks](./docs/benchmarks.md#speed)).

KIPA is still a work in progress, and should not be used for any serious cases.
It is recommended that you generate a new key to try out KIPA. For a list of
unimplemented planned features, see [here](./docs/todo.md).

## How does it work?

When a node joins the KIPA network, its public key is mapped to an
_n_-dimensional space, where _n_ is constant throughout the network. The mapping
is done uniformly and deterministically. This space is called **key space**.

Each node will attempt to find the IP addresses of other nodes which are close
to it in key space, and set those nodes to be its **neighbours**. Once this is
achieved, look-ups can be performed using simple graph search algorithms: a node
can map a chosen public key into key space, and then identify its neighbour
closest to that point, and ask that neighbour for _its_ closest neighbour to
that point. The process continues until the correct node is found (or until it
is determined that no such node exists). This is a slightly modified _greedy
best-first search_ algorithm, where the metric is distance in key space.

Nodes connect to the network through an initial node - this node can be any node
in the network, but its IP address and public key must be known before
connecting. In order to find its neighbours, the connecting node performs a
search for itself in the network (in a similar style to above), and selects the
closest nodes it encounters.

You can find a more detailed overview of KIPA's design
[here](./docs/design.md).

## Why does it exist?

KIPA is a tool for use in distributed systems. It can replace DNS in scenarios
where DNS isn't appropriate - for example, when:
- The IP addresses of nodes change often.
- There are too many nodes to enrol in DNS registrars.
- Deploying distributed systems with community nodes, where community enrollment
  is difficult.
- DNS does not guarantee high enough security.

It can also be used for casual cases, for example sending files between
computers when IP addresses are not known, but public keys are:
```bash
# Run on receiver
nc -l -p 8080 > file.txt
# Run on sender
cat file.txt > nc $(kipa search --key-id $RECEIVER_KEY_ID --print ip) 8080
```

Any use of KIPA requires that keys are already known in the system - it does
not solve the problem of key distribution. What it does do is provide a secure
and distributed infrastructure for resolving up-to-date IP addresses.

### Why distributed?

Distributed systems have several advantages over centralised ones. In the case
of KIPA, some specific advantages arise from its distributed architecture:
- **Privacy**: As messages are spread evenly throughout the network, no single
  node sees all messages. Therefore, total information control is impossible to
  achieve unless all nodes are controlled by one organisation\*.
- **Robustness**: No single node can fail and corrupt the entire network.
- **Community control**: Control of the network is not given to one
  organisation, meaning that the performance and stability of the network is
  dependent upon the community. If the community uses KIPA, KIPA stays up.
  Alternatively if no one does, KIPA goes down.

\* This design has the effect that each node is aware of a portion of the
look-ups in the network. However, as the amount of nodes in the network
increases, this portion becomes smaller and smaller. Therefore, no significant
amount of information is seen by any single node.

## Usage

Prerequisites:
- Rust and Cargo >= 1.26.0
- Protobuf compiler >= 3.5.1
- GnuPG >= 2.2.8
  - `gpgme` crate requires `autogen`, `gettext` to build

```bash
# Download and build
git clone https://github.com/mishajw/kipa.git && cd kipa
cargo build --release

# Run KIPA daemon
./target/release/kipa-daemon --key-id $YOUR_KEY_ID &

# Connect to a KIPA network
./target/release/kipa connect \
    --key-id <root key ID> --address <root address>

# Example query
./target/release/kipa search \
    --key-id $THEIR_KEY_ID

# Run tests
cargo test
```

### Simulations
KIPA network simulation code is found in `./simulation`. This also includes
end-to-end tests, and benchmarking. Simulation results are written to
`./simulation_output`.

The [benchmarks](./docs/benchmarks.md) document discusses how the simulations
are used to evaluate the performance of KIPA.

The simulations create a network of Docker containers. All created resources are
prefixed with `kipa_simulation_` and are removed after the simulation is
finished.

Prerequisites:
- All previously mentioned prerequisites
- Python and Pip >= 3.6
- `virtualenv` >= 15.1.0
- Docker >= 18.05.0, with daemon running

```bash
# Install dependencies in virtualenv
python -m venv .env && source .env/bin/activate
pip install -r simulation/requirements.txt

# Run end-to-end tests
python -m unittest discover simulation

# Run simulation configuration
# Example network configurations exist in `./resources/simulaton_configs/`
python -m simulation --network_config $NETWORK_CONFIGURATION_FILE
```
