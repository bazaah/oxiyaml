#![allow(dead_code)]

mod error;
mod event;
mod machine;
mod node;
mod scanner;
mod states;

use std::io;

use crate::{
    error::{Error, ErrorKind, Result},
    event::Event,
    machine::*,
    node::NodeKind,
    scanner::*,
    states::*,
};

/// State machine handle, this struct operates the state machine
/// and exposes an higher level interface
struct Handle<R> {
    machine: State<io::Bytes<R>>,
}

impl<R> Handle<R>
where
    R: io::Read,
{
    // Initialize a new parse handle
    fn new(stream: R) -> Self {
        Self {
            machine: State::new(stream.bytes()),
        }
    }

    /// Cycles the state machine, returning the next YAML node
    fn next_node(&mut self) -> Option<Result<NodeKind>> {
        // Early returns from this function must ensure that the
        // machine is returned to 'self' before returning from the function

        // machine is taken here
        let mut machine = std::mem::replace(&mut self.machine, State::Dummy);

        let node = loop {
            let mut output = None;
            machine = machine.step(&mut output);

            if let Some(event) = output {
                break event.transpose();
            }
        };

        // returned here
        std::mem::swap(&mut self.machine, &mut machine);

        node
    }
}

impl<R: io::Read> Iterator for Handle<R> {
    type Item = std::result::Result<NodeKind, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_node()
    }
}

/// Contains all legal states of the machine
/// and controls the legal transitions between them
enum State<I> {
    Start(StateMachine<I, Start>),
    LineStart(StateMachine<I, LineStart, Active>),
    LineEnd(StateMachine<I, LineEnd>),

    // Ambiguous
    AmbiguousScalar(StateMachine<I, AmbiguousScalar>),
    AmbiguousColon(StateMachine<I, AmbiguousColon>),

    // Scalar
    ScalarLiteral(StateMachine<I, ScalarLiteral>),

    // Map
    MapStart(StateMachine<I, MapStart>),
    MapVerifyKey(StateMachine<I, MapVerifyKey>),
    MapWhiteSpace(StateMachine<I, MapWhiteSpace>),
    MapValue(StateMachine<I, MapValue>),

    // Exit
    Done(StateMachine<I, Done>),
    Failure(StateMachine<I, Failure>),

    // Dummy type
    Dummy,
}

impl<I> State<I>
where
    I: Iterator<Item = io::Result<u8>>,
{
    /// Initialize new binding
    fn new(stream: I) -> Self {
        Self::Start(StateMachine::new(stream))
    }

    /// Moves the machine forward one step.
    /// Each state's driver is passed an event handle
    /// which it can use to return collected output (if any.)
    fn step(self, output: &mut Event) -> Self {
        match self {
            Self::Start(mut st) => match st.drive(output) {
                Ok(Marker::LineStart) => Self::LineStart(st.into()),
                Ok(Marker::Done) => Self::Done(st.into()),
                Err(e) => Self::Failure((e, st).into()),
                _ => Self::Failure((ErrorKind::IllegalTransition.into(), st).into()),
            },
            Self::LineStart(mut st) => match st.drive(output) {
                Ok(Marker::LineEnd) => Self::LineEnd(st.into()),
                Ok(Marker::AmbiguousScalar) => Self::AmbiguousScalar(st.into()),
                Ok(Marker::Done) => Self::Done(st.into()),
                _ => Self::Failure((ErrorKind::IllegalTransition.into(), st).into()),
            },
            Self::LineEnd(mut st) => match st.drive(output) {
                Ok(Marker::LineStart) => Self::LineStart(st.into()),
                Ok(Marker::Done) => Self::Done(st.into()),
                Err(e) => Self::Failure((e, st).into()),
                _ => Self::Failure((ErrorKind::IllegalTransition.into(), st).into()),
            },
            // =====================
            Self::AmbiguousScalar(mut st) => match st.drive(output) {
                Ok(Marker::AmbiguousColon) => Self::AmbiguousColon(st.into()),
                Ok(Marker::ScalarLiteral) => Self::ScalarLiteral(st.into()),
                Err(e) => Self::Failure((e, st).into()),
                _ => Self::Failure((ErrorKind::IllegalTransition.into(), st).into()),
            },
            Self::AmbiguousColon(mut st) => match st.drive(output) {
                Ok(Marker::AmbiguousScalar) => Self::AmbiguousScalar(st.into()),
                Ok(Marker::MapStart) => Self::MapStart(st.into()),
                Err(e) => Self::Failure((e, st).into()),
                _ => Self::Failure((ErrorKind::IllegalTransition.into(), st).into()),
            },
            // ======================
            Self::ScalarLiteral(mut st) => match st.drive(output) {
                Ok(Marker::Done) => Self::Done(st.into()),
                Err(e) => Self::Failure((e, st).into()),
                _ => Self::Failure((ErrorKind::IllegalTransition.into(), st).into()),
            },
            // ======================
            Self::MapStart(mut st) => match st.drive(output) {
                Ok(Marker::MapVerifyKey) => Self::MapVerifyKey(st.into()),
                Err(e) => Self::Failure((e, st).into()),
                _ => Self::Failure((ErrorKind::IllegalTransition.into(), st).into()),
            },
            Self::MapVerifyKey(mut st) => match st.drive(output) {
                Ok(Marker::MapWhiteSpace) => Self::MapWhiteSpace(st.into()),
                Err(e) => Self::Failure((e, st).into()),
                _ => Self::Failure((ErrorKind::IllegalTransition.into(), st).into()),
            },
            Self::MapWhiteSpace(mut st) => match st.drive(output) {
                Ok(Marker::MapValue) => Self::MapValue(st.into()),
                Err(e) => Self::Failure((e, st).into()),
                _ => Self::Failure((ErrorKind::IllegalTransition.into(), st).into()),
            },
            Self::MapValue(mut st) => match st.drive(output) {
                Ok(Marker::LineEnd) => Self::LineEnd(st.into()),
                Ok(Marker::Done) => Self::Done(st.into()),
                Err(e) => Self::Failure((e, st).into()),
                _ => Self::Failure((ErrorKind::IllegalTransition.into(), st).into()),
            },
            // ======================
            Self::Done(st) => Self::Done(st.cycle(output)),
            Self::Failure(st) => Self::Failure(st.cycle(output)),

            // ======================
            Self::Dummy => panic!("Attempted to use a dummy state... this is a bug"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{io::Cursor, str::from_utf8};

    #[test]
    fn key_plain() -> Result<()> {
        let data = Cursor::new(include_str!("../testing/data/key-plain.yaml"));
        let handle = Handle::new(data);

        handle
            .into_iter()
            .take(100)
            .try_for_each(|node: Result<NodeKind>| {
                match node? {
                    NodeKind::Key(v) | NodeKind::ScalarPlain(v) => {
                        println!("Value: {:?}", from_utf8(&v))
                    }
                }

                Ok(())
            })
    }

    #[test]
    fn key_squote() -> Result<()> {
        let data = Cursor::new(include_str!("../testing/data/key-squote.yaml"));
        let handle = Handle::new(data);

        handle
            .into_iter()
            .take(100)
            .try_for_each(|node: Result<NodeKind>| {
                match node? {
                    NodeKind::Key(v) | NodeKind::ScalarPlain(v) => {
                        println!("Value: {:?}", from_utf8(&v))
                    }
                }

                Ok(())
            })
    }

    #[test]
    fn key_dquote() -> Result<()> {
        let data = Cursor::new(include_str!("../testing/data/key-dquote.yaml"));
        let handle = Handle::new(data);

        handle
            .into_iter()
            .take(100)
            .try_for_each(|node: Result<NodeKind>| {
                match node? {
                    NodeKind::Key(v) | NodeKind::ScalarPlain(v) => {
                        println!("Value: {:?}", from_utf8(&v))
                    }
                }

                Ok(())
            })
    }

    #[test]
    fn map_plain() -> Result<()> {
        let data = Cursor::new(include_str!("../testing/data/map-plain.yaml"));
        let handle = Handle::new(data);

        handle
            .into_iter()
            .take(100)
            .try_for_each(|node: Result<NodeKind>| {
                match node? {
                    NodeKind::Key(v) | NodeKind::ScalarPlain(v) => {
                        println!("Value: {:?}", from_utf8(&v))
                    }
                }

                Ok(())
            })
    }

    #[test]
    fn sequence_plain() -> Result<()> {
        let data = Cursor::new(include_str!("../testing/data/sequence-plain.yaml"));
        let handle = Handle::new(data);

        handle
            .into_iter()
            .take(100)
            .try_for_each(|node: Result<NodeKind>| {
                match node? {
                    NodeKind::Key(v) | NodeKind::ScalarPlain(v) => {
                        println!("Value: {:?}", from_utf8(&v))
                    }
                }

                Ok(())
            })
    }
}
