# Design

This document will discuss the design of KIPA. Please read the
[README.md](../README.md) first. Implementation documentation can be found at
https://docs.rs/kipa.

## High-level design overview

Two binaries are produced by the source: the command line interface (CLI) and
the daemon. The CLI will pass on user requests to the daemon. The daemon is
designed to run in the background, and will listen for requests from both the
CLI and other KIPA nodes.

Both binaries have a shared library, `kipa_lib`. This is where most code exists,
including API definitions and servers/clients for communication from
daemon-to-daemon and CLI-to-daemon.

### Components

All components exist in `kipa_lib`:
- **`address`**, **`key`**, **`node`**: Building blocks: the IP addresses, GPG
  keys, and the nodes which have an address and a key.
- **`api::*`**: Defines the API for communicating between nodes. Also used for
  sending messages between the daemon and CLI.
- **`server::{Server,Client}`**: Implementers are responsible for listening for
  requests from other nodes, or sending requests to other nodes.
- **`server::{LocalServer,LocalClient}`**: Implementers are responsible for
  listening for requests from the CLI, or sending requests from a CLI to a
  daemon.
- **`data_transformer::DataTransformer`**: Implementers are responsible for
  converting raw bytes into a `api::Message`.
- **`message_handler::MessageHandler`**: Receives an `api::RequestMessage` and
  returns an `api::ResponseMessage`.
- **`payload_handler::PayloadHandler`**: Implementers are responsible for
  receiving an `api::RequestPayload` and replying with an
  `api::ResponsePayload`.
- **`error::*`**: Defines internal and public-facing errors used across the
  project.

### Request control flow

This section describes the control flow from receiving a request, to replying
with a response:
- `Server` receives a request on a listening port (or similar
  mechanism) and passes the decoded message to a `MessageHandler`.
- `MessageHandler` is responsible for:
  - Decodes the raw bytes using a `DataTransformer`, to get sender information
    and encryped payload.
  - Decrypting the payload, and decoding it using a `DataTransformer`.
  - Setting the correct message identifier once the reply has been created.
  - Creating an interface for sending messages to other nodes, in order for the
    `PayloadHandler` to be able to perform queries.
- `PayloadHandler` reads the payload, performs any tasks described by the
  payload, and returns a response.
- The message is passed back through the `MessageHandler` to the `Server` which
  replies to the original sender.

## API

The API is described in the source file for the module
[`kipa_lib::api`](https://docs.rs/kipa/*/kipa_lib/api/index.html).

## Payload handling

`kipa_lib::payload_handler::graph` contains the main implementation of
`PayloadHandler`. This implementation is aware of the key for the local node,
and remembers the closest (in key space) encountered nodes to this key. These
nodes are its **neighbours** - or when talking about graphs, its **edges**.

The implementation handles search requests by performing graph searches on the
network. It starts from the local node and is therefore aware of its own
neighbours/edges.  The graph search is detailed [here](#graph-search). It
handles connect requests by performing a search for itself.

The search failure condition and the connection finishing condition are
equivalent, although with potentially different parameters. This condition is
that the _n_ discovered closest nodes to the destination (for search it is the
search key, for connect it is the local key) have all been queried for their
neighbours. The intuition for why this works is that once all closest nodes have
been queried and *they* do not know any closer nodes, then they must be the
closest nodes.

### Graph search

The search algorithm used is greedy best-first search. It has some key
modifications:
1. It runs in parallel, spawning jobs to query nodes. This does not change the
   result of the algorithm, but changes its structure.
2. The exit condition is determined by callbacks. The return value of the
   callbacks determines whether to continue, finish, or fail the search. There
   are two callbacks used:
   1. `found_node_callback`: called when a node has been found as the neighbour
      of another node.
   2. `explored_node_callback`: called when a node has been queried for its
      neighbours.

The modified algorithm is described here:
1. Set up data structures:
   1. Set `to_explore` to contain initial node(s).
   2. Set `found` to empty.
   3. Set up `explored_channel` for communicating nodes explored/found by
      threads.
2. Consume from `explored_channel` until empty, each explored/found node is
   passed to `{explored,found}_node_callback` with the option to exit the
   search.
3. Check conditions:
   1. If `num_threads == 0 && to_explore.empty()`, then exit.
   2. If `num_threads > 0 && to_explore.empty()`, then wait for thread to finish
      and before going to step 2.
   3. If `num_threads >= max_threads`, then wait for threads to finish before
      going step 2.
   4. If `num_threads < max_threads`, then continue.
4. Pop node off `to_explore`, prioritised by key space distance to destination.
5. Spawn thread for exploring popped node, which does:
   1. Ask node for neighbours.
   2. Send the explore node and found nodes through `explored_channel`.
6. Go to step 2.

### Selecting neighbours

Whenever a node is encountered during a search or connection, it becomes a
candidate neighbour. This section describes how candidates are selected to
become neighbours.

Each node has a fixed maximum amount of neighbours it can hold. This is
configured by the user, as it is dependent on how much spare memory there is on
the machine. Each neighbour can, in the worst case, have an IPv6 address and a
4096 bit key. This will take up approximately 4226 bytes (128 for IP + 4096 for
key + 2 for port). This means that with a megabyte of memory, a node can store
approximately 250 neighbours. But how do we select which neighbours to store,
and which neighbours to discard?

In this document we have described selecting the only closest neighbours. This
is not the entire truth. This section will describe how selection is done in
more detail.

When selecting neighbours we have three goals:
1. **Each node's selected neighbours should be as predictable as possible.**
   This is so graph searches can determine what neighbours a node might have,
   and therefore choose whether or not to query the node.
2. **The graph should be fully connected**: In order to be fully connected,
   there must be a path from every node to every other node.
3. **Reduce the average path length of the graph.** When performing a search, it
   is important that the average number of steps between any two nodes is as
   small as possible. There will be at least one request made for every step in
   the path, and therefore the shorter the path, the quicker the query can be
   made.

Goal 1 is achieved by selecting neighbours that are closest to a node in key
space.  This allows the graph search algorithms to work. However, minimising
distance alone does not guarantee goal 2 - good connectivity.

KIPA improves on goal 2 by also trying to increase the *distribution of angles*
of a node's neighbours. This means that when considering new neighbours, we will
prioritise ones that add an edge in a new direction.

**WIP:** A possible approach to goal 3 is to introduce randomness to neighbour
selection. Whenever a new neighbour is encountered, they go through the normal
neighbour selection process, but also have a small probability of being selected
for a neighbour regardless of the result. The intuition is that this will add
long links across key space that decrease the average distance between two
nodes.

**WIP:** A possible approach to goal 3 is to also assign each node a **radius**
in the same way key space coordinates are assigned. Instead of prioritising
neighbours that are closest to a node, we prioritise neighbours that are closest
to the radius around the node (e.g. if the radius is 5 units, we select
neighbours that are 5 units away from us over ones that are 1 unit away). Then,
when performing a graph search, we will query nodes that have the closest radius
to our destination. The intuition is that the graph will have predictable "high
ways" allowing our searches to jump across key space in short amounts of time.

## Security design

Security is a major concern in the development of KIPA, as [prior
mistakes](https://en.wikipedia.org/wiki/DNS_spoofing) in IP address resolution
have proven to be extremely exploitable. This section will discuss some security
concerns and how KIPA deals with them. Any concerns that are not addressed in
this section are welcome to be brought up as an
[issue](https://github.com/mishajw/kipa/issues/new).

However, it should be noted that KIPA relies on the public key of a node being
known prior to any search for that node. This means that many security
guarantees are inherent, especially relating to authenticity and secrecy.

### Communication protocol

**WIP:** The security of the communication protocol relies on public key
encryption and signatures.

Each request and response message has:
- The message sender, including:
  - The port that the daemon is listening on (while the IP address is inferred
    from the connection).
  - The sender's public key.
- A signature of the decrypted message content, signed by the sender's private
  key.
- The message content, encrypted with the recipient's public key, containing:
  - The message identifier.
  - The payload of the message.

The **signature** ensures that the message has come from the correct sender.
This provides **authenticity**.

The **encryption** of the message content ensures that the message can only be
read by the recipient. This provides **secrecy**.

The **message identifier** is encrypted in the message content, and is verified
when a response is received. This provides assurance that the reply **comes from
the recipient**, as only the recipient can see the identifier. This also
prevents **replay attacks**.

### Verified key look-up

**WIP:** Key look-ups are guaranteed to only succeed if the IP address actually
belongs to the searched key. This is because at the end of each search, a
verification message is sent to the node. The verification message contains an
empty payload, but still contains a message identifier. For a valid reply, the
receiver will have to both decrypt the message identifier using their private
key, and sign the message identifier using their private key. This allows the
sending node to verify that the IP address does belong to the correct key.

It may seem that signing IP addresses would be preferable to verification
messages: if each node has the signatures for each of its neighbours, it can be
assured that (at least at some point) these neighbours were listening on these
IPs, and each search operation would need one less request. However, it is
difficult to verify what IP address a node is listening on, due to NAT and
requests leaving from different interfaces (and therefore different IPs). The
verification message also provides an up-to-date verification, and prevents
attacks which involve taking over an IP address after they have been signed.

### Zero-trust

KIPA is a zero-trust network. This means that no node completely relies on the
information received from another node - it relies on information from several
different nodes. When searching, a node will query several different nodes
simultaneously: if one of them returns corrupt information, the search will
still succeed as long as the information returned from one of the nodes is
correct.

### Remaining a fully distributed system

A common problem with distributed systems is that while they may start with
several independent nodes, eventually users of the system will start to only use
a select few nodes. This results in the system essentially becoming centralised,
and therefore losing all the benefits of being a distributed system. Users
usually do this because of the inconvenience of setting up a node themselves.
This is what has happened with IRC (with only a couple of hundred major active
IRC servers) and Bitcoin (with few organisations controlling the majority of
mining resources).

KIPA does not have this problem: In order for an individual to use the system,
it is required that they become a node in the network. As every node is equal,
there will be as many active nodes as there are active users.
