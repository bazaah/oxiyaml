#![allow(dead_code)]

mod machine;
mod node;
mod scanner;
mod states;
mod error;
mod event;

use std::io;

use crate::{machine::*, scanner::*, states::*, node::NodeKind, error::{Result, Error, ErrorKind}, event::Event};

const SAMPLE: &str = 
/* Formatting */
r#"zero
    four: a mapping value
        eight
            twelve
    four
zero
"#;

struct Handle<R> {
    machine: State<io::Bytes<R>>,
}

impl<R> Handle<R>
where
    R: io::Read,
{
    fn new(stream: R) -> Self {
        Self {
            machine: State::new(stream.bytes()),
        }
    }

    fn next_node(&mut self) -> Option<Result<NodeKind>> {
        let mut machine = std::mem::replace(&mut self.machine, State::Dummy);
        
        let node = loop {
            let mut output = None;
            machine = machine.step(&mut output);
            
            if let Some(event) = dbg!(output) {
                break event.transpose()
            }
        };

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
    fn new(stream: I) -> Self {
        Self::Start(StateMachine::new(stream))
    }

    fn step(self, o: &mut Event) -> Self {
        match self {
            Self::Start(mut st) => match st.drive(o) {
                Ok(Marker::LineStart) => Self::LineStart(st.into()),
                Ok(Marker::Done) => Self::Done(st.into()),
                Err(e) => Self::Failure((e, st).into()),
                _ => Self::Failure((ErrorKind::IllegalTransition.into(), st).into()),
            },
            Self::LineStart(mut st) => match st.drive(o) {
                Ok(Marker::LineEnd) => Self::LineEnd(st.into()),
                Ok(Marker::AmbiguousScalar) => Self::AmbiguousScalar(st.into()),
                Ok(Marker::Done) => Self::Done(st.into()),
                _ => Self::Failure((ErrorKind::IllegalTransition.into(), st).into()),
            },
            Self::LineEnd(mut st) => match st.drive(o) {
                Ok(Marker::LineStart) => Self::LineStart(st.into()),
                Ok(Marker::Done) => Self::Done(st.into()),
                Err(e) => Self::Failure((e, st).into()),
                _ => Self::Failure((ErrorKind::IllegalTransition.into(), st).into()),
            },
            // =====================
            Self::AmbiguousScalar(mut st) => match st.drive(o) {
                Ok(Marker::AmbiguousColon) => Self::AmbiguousColon(st.into()),
                Ok(Marker::ScalarLiteral) => Self::ScalarLiteral(st.into()),
                Err(e) => Self::Failure((e, st).into()),
                _ => Self::Failure((ErrorKind::IllegalTransition.into(), st).into()),
            }
            Self::AmbiguousColon(mut st) => match st.drive(o) {
                Ok(Marker::AmbiguousScalar) => Self::AmbiguousScalar(st.into()),
                Ok(Marker::MapStart) => Self::MapStart(st.into()),
                Err(e) => Self::Failure((e, st).into()),
                _ => Self::Failure((ErrorKind::IllegalTransition.into(), st).into()),
            }
            // ======================
            Self::ScalarLiteral(mut st) => match st.drive(o) {
                Ok(Marker::Done) => Self::Done(st.into()),
                Err(e) => Self::Failure((e, st).into()),
                _ => Self::Failure((ErrorKind::IllegalTransition.into(), st).into()),
            }
            // ======================
            Self::MapStart(mut st) => match st.drive(o) {
                Ok(Marker::MapVerifyKey) => Self::MapVerifyKey(st.into()),
                Err(e) => Self::Failure((e, st).into()),
                _ => Self::Failure((ErrorKind::IllegalTransition.into(), st).into()),
            }
            Self::MapVerifyKey(mut st) => match st.drive(o) {
                Ok(Marker::MapWhiteSpace) => Self::MapWhiteSpace(st.into()),
                Err(e) => Self::Failure((e, st).into()),
                _ => Self::Failure((ErrorKind::IllegalTransition.into(), st).into()),
            }
            Self::MapWhiteSpace(mut st) => match st.drive(o) {
                Ok(Marker::MapValue) => Self::MapValue(st.into()),
                Err(e) => Self::Failure((e, st).into()),
                _ => Self::Failure((ErrorKind::IllegalTransition.into(), st).into()),
            }
            Self::MapValue(mut st) => match st.drive(o) {
                Ok(Marker::LineEnd) => Self::LineEnd(st.into()),
                Ok(Marker::Done) => Self::Done(st.into()),
                Err(e) => Self::Failure((e, st).into()),
                _ => Self::Failure((ErrorKind::IllegalTransition.into(), st).into()),
            }
            // ======================
            Self::Done(st) => Self::Done(st.cycle(o)),
            Self::Failure(st) => Self::Failure(st.cycle(o)),

            // ======================
            Self::Dummy => panic!("Logic error, this is a bug"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{io::Cursor, str::from_utf8};

    #[test]
    fn check_state_machine() {
        let data = Cursor::new("test: value one:testing");
        let handle = Handle::new(data);

        handle.into_iter().filter_map(|res| res.or_else(|e| Err(println!("Error! {}", e))).ok()).take(100).for_each(|node| {
            match node {
                NodeKind::Key(v) | NodeKind::ScalarPlain(v) => println!("Value: {:?}", from_utf8(&v))
            }
        });

        panic!();
    }
}
