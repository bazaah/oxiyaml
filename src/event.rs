use super::{node::NodeKind, Error, Result};

/// Type def for an event
pub(super) type Event = Option<EventKind>;

/// Describes the possible events that can occur
#[derive(Debug)]
pub(super) enum EventKind {
    Node(NodeKind),
    Failure(Error),
    Done,
}

impl EventKind {
    /// Converts an event into the equivalent Option/Result nesting
    pub(super) fn transpose(self) -> Option<Result<NodeKind>> {
        match self {
            Self::Node(node) => Some(Ok(node)),
            Self::Failure(err) if err.is_repeat() => None,
            Self::Failure(err) => Some(Err(err)),
            Self::Done => None,
        }
    }
}

impl From<NodeKind> for EventKind {
    fn from(node: NodeKind) -> Self {
        Self::Node(node)
    }
}

impl From<Error> for EventKind {
    fn from(err: Error) -> Self {
        Self::Failure(err)
    }
}

impl From<()> for EventKind {
    fn from(_: ()) -> Self {
        Self::Done
    }
}
