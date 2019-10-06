use api::{Key, Node};
use error::*;

/// Callbacks when performing searches.
pub trait SearchCallback<T>: Send + Sync + 'static {
    /// Gets the neighbours of a node, closest to a search key.
    fn get_neighbours(&self, node: &Node, search_key: &Key) -> InternalResult<Vec<Node>>;

    /// Notifies that a node has been found, i.e. we have become aware the node exists.
    fn found_node(&self, node: &Node) -> Result<SearchCallbackAction<T>>;

    /// Notifies that a node has been explored, i.e. the node has been successfully queried for its
    /// neighbours.
    fn explored_node(&self, node: &Node) -> Result<SearchCallbackAction<T>>;
}

/// Actions to perform as the result of a callback.
pub enum SearchCallbackAction<T> {
    /// No action, continues the search.
    Continue(),
    /// Successfully finish the search, returning a value.
    Return(T),
    /// Unsuccessfully finish the search.
    Exit(),
}

/// Executes the `SearchCallbackAction`.
///
/// Not a regular function, as we want to propagate the `return` statements.
macro_rules! execute_callback_action {
    ($callback_value:expr) => {
        match $callback_value {
            SearchCallbackAction::Continue() => {}
            SearchCallbackAction::Return(t) => return Ok(Some(t)),
            SearchCallbackAction::Exit() => return Ok(None),
        }
    };
}
