use super::{node::NodeKind, Error, Result};

pub(super) type Event = Option<EventKind>;

#[derive(Debug)]
pub(super) enum EventKind {
    Node(NodeKind),
    Failure(Error),
    Done,
}

impl EventKind {
    pub(super) fn transpose(self) -> Option<Result<NodeKind>> {
        match self {
            Self::Node(node) => Some(Ok(node)),
            Self::Failure(err) if err.has_failed() => None,
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

// pub(super) trait Transition<T>: Sized {
//     type Output;

//     fn transition(_: T, _: &mut Self::Output) -> Self;
// }

// pub(super) trait TransitionInto<T>: Sized {
//     type Output;

//     fn transform(self, _: &mut Self::Output) -> T;
// }

// impl<T, U> TransitionInto<U> for T
// where
//     U: Transition<T>,
// {
//     type Output = U::Output;
//     fn transform(self, o: &mut Self::Output) -> U {
//         U::transition(self, o)
//     }
// }
