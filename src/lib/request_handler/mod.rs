use error::*;
use key::Key;
use node::Node;

#[cfg(feature = "use-graph")]
pub mod graph;

pub enum Request {
    SearchRequest(Key),
    QueryRequest(Key)
}

pub enum Response {
    SearchResponse(Node),
    QueryResponse(Vec<Node>)
}

pub trait RequestHandler: Send + Sync {
    fn receive(&self, req: &Request) -> Result<Response>;
}

