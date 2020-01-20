# Design

This document will discuss the design of KIPA. Please read the
[README.md](../README.md) first. Implementation documentation can be found at
https://docs.rs/kipa.

## High-level design overview

Two binaries are produced by the source: the `kipa` command line interface
(CLI) and the daemon, `kipa-daemon`. The CLI will pass on user requests to the
daemon. The daemon is designed to run in the background, and will listen for
requests from both the CLI and other KIPA nodes.

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
- **`message_handler::MessageHandlerServer`**: Receives an
  `api::RequestMessage` from another daemon or CLI and returns an
  `api::ResponseMessage` to the daemon or CLI.
- **`message_handler::MessageHandlerClient`**: Sends an `api::RequestMessage`
  to another daemon and returns the daemon's `api::ResponseMessage`
- **`payload_handler::PayloadHandler`**: Implementers are responsible for
  receiving an `api::RequestPayload` and replying with an
  `api::ResponsePayload`.
- **`error::*`**: Defines internal and public-facing errors used across the
  project.

### Request control flow

This section describes the control flow from receiving a request, to replying
with a response:
- `Server` receives a request on a listening port (or similar mechanism) and
  will pass the raw bytes message to a `MessageHandlerServer`.
- `MessageHandlerServer` will:
  - Decode the raw bytes using a `DataTransformer`, to get sender information
    and encryped payload.
  - Decrypt the payload, and decode it using a `DataTransformer`.
  - Set the correct message identifier once the reply has been created.
  - Create an `MessageHandlerClient` for sending messages to other nodes, in
    order for the `PayloadHandler` to be able to perform queries.
- `PayloadHandler` will read the payload, and perform any tasks described by
  the payload, and return a response.
- The message is passed back through the `IncomingMessageHandler` to the
  `Server` which replies to the original sender.

## API

The API is described in the source file for the module
[`kipa_lib::api`](https://docs.rs/kipa/*/kipa_lib/api/index.html).

## Messaging protocol

Requests are defined as:
- Request body (encrypted for recipient, signed by sender), containing:
  - Message ID (randomly generated).
  - Request payload.
- Sender's public key.

Responses are defined as:
- Response body (encrypted for sender, signed by recipient), containing:
  - Message ID (identical to request's message ID).
  - Response payload.

This protocol provides the following guarantees:
- **All queries are only observed by the recipient and sender.** Ensured by
  asymmetrically encrypted request and response bodies.
- **Requests have not been modified in transit.** Ensured by signed request
  bodies.
- **Responses are from the recipient.** Ensured by signed response bodies.
- **Responses aren't replayed.** Ensured by message ID in the request.

## Payload handling

`kipa_lib::payload_handler::graph` contains the main implementation of
`PayloadHandler`. This implementation is aware of the key for the local node,
and remembers the closest (in key space) encountered nodes to this key. These
nodes are its **neighbours** - or when talking about graphs, its **edges**.

The implementation handles search requests by performing graph searches on the
network. It starts from the local node and is therefore aware of its own
neighbours/edges. The graph search is detailed [here](#graph-search). It
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
1. It runs in parallel, using a thread pool to query nodes. This does not
   change the result of the algorithm, but changes its structure.
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

Each node has a fixed maximum amount of neighbours it can hold, _n_. This is
configured by the user, as it is dependent on how much spare memory there is on
the machine. The KIPA daemon will store the IP address and key of the _n_ nodes
closest to itself in key space.

Each neighbour can, in the worst case, have an IPv6 address and a 4096 bit key.
This will take up approximately 4226 bytes (128 for IP + 4096 for key + 2 for
port). This means that with a megabyte of memory, a node can store
approximately 250 neighbours.

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

The security of the communication protocol relies on public key encryption and
signatures.

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

Key look-ups are guaranteed to only succeed if the IP address actually belongs
to the searched key. This is because at the end of each search, a verification
message is sent to the node. The verification message contains an empty
payload, but still contains a message identifier. For a valid reply, the
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

## Release process

1. Check out the `release` branch.
2. Rebase the `release` branch onto `origin/master`. This should contain a single commit that adds
   the autogenerated protobuf code, and removes its generation in `build.rs`.
3. Bump the release number, using semver releases. Don't commit this yet.
4. Run `cargo publish`.
5. Once successful, commit the new release number to `origin/master`.
6. Add a tag to this new commit, e.g. `git tag v1.2.3`.
7. Push the release commit and tag.