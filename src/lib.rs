#![allow(dead_code)]

mod machine;
mod node;
mod scanner;
mod states;
mod error;

pub(self) use crate::{machine::*, scanner::*, states::*, node::NodeKind, error::{Result, Error, ErrorKind}};

const SAMPLE: &str = 
/* Formatting */
r#"zero
    four: a mapping value
        eight
            twelve
    four
zero
"#;

struct Handle<I> {
    machine: State<I>,
}

impl<I> Handle<I>
where
    I: Iterator<Item = u8>,
{
    fn new(stream: I) -> Self {
        Self {
            machine: State::new(stream),
        }
    }

    fn next_node(&mut self) -> Result<NodeKind> {
        let mut machine = std::mem::replace(&mut self.machine, State::Dummy);
        
        let node = loop {
            let mut output = None;
            machine = machine.step(&mut output);
            
            if let Some(res) = output.map(|node| match node {
                NodeKind::Failure(err) => Err(err),
                other => Ok(other),
            }) {
                break res;
            }
        };

        std::mem::swap(&mut self.machine, &mut machine);

        node
    } 
}

enum State<I> {
    Start(StateMachine<I, Start>),
    WhiteSpace(StateMachine<I, WhiteSpace>),
    Ignore(StateMachine<I, Ignore>),
    LineStart(StateMachine<I, LineStart, Active>),
    LineEnd(StateMachine<I, LineEnd>),
    Done(StateMachine<I, Done>),
    Failure(StateMachine<I, Failure>),

    // Dummy type
    Dummy,
}

impl<I> State<I>
where
    I: Iterator<Item = u8>,
{
    fn new(stream: I) -> Self {
        Self::Start(StateMachine::new(stream))
    }

    fn step(self, o: &mut Option<NodeKind>) -> Self {
        match self {
            Self::Start(mut st) => match st.marker() {
                Ok(Marker::LineStart) => Self::LineStart(st.transform(o)),
                Ok(Marker::Done) => Self::Done(st.transform(o)),
                Err(e) => Self::Failure((e, st).transform(o)),
                _ => Self::Failure((ErrorKind::IllegalTransition.into(), st).transform(o)),
            },
            Self::WhiteSpace(mut st) => match st.marker() {
                Ok(Marker::LineEnd) => Self::LineEnd(st.transform(o)),
                Ok(Marker::Ignore) => Self::Ignore(st.transform(o)),
                Ok(Marker::Done) => Self::Done(st.transform(o)),
                Err(e) => Self::Failure((e, st).transform(o)),
                _ => Self::Failure((ErrorKind::IllegalTransition.into(), st).transform(o)),
            },
            Self::Ignore(mut st) => match st.marker() {
                Ok(Marker::LineEnd) => Self::LineEnd(st.transform(o)),
                Ok(Marker::WhiteSpace) => Self::WhiteSpace(st.transform(o)),
                Ok(Marker::Done) => Self::Done(st.transform(o)),
                Err(e) => Self::Failure((e, st).transform(o)),
                _ => Self::Failure((ErrorKind::IllegalTransition.into(), st).transform(o)),
            },
            Self::LineStart(mut st) => match st.marker() {
                Ok(Marker::LineEnd) => Self::LineEnd(st.transform(o)),
                Ok(Marker::Ignore) => Self::Ignore(st.transform(o)),
                Ok(Marker::Done) => Self::Done(st.transform(o)),
                _ => Self::Failure((ErrorKind::IllegalTransition.into(), st).transform(o)),
            },
            Self::LineEnd(mut st) => match st.marker() {
                Ok(Marker::LineStart) => Self::LineStart(st.transform(o)),
                Ok(Marker::Done) => Self::Done(st.transform(o)),
                Err(e) => Self::Failure((e, st).transform(o)),
                _ => Self::Failure((ErrorKind::IllegalTransition.into(), st).transform(o)),
            },
            st @ Self::Done(_) => st,
            st @ Self::Failure(_) => st,
            Self::Dummy => panic!("Logic error, this is a bug"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // #[test]
    // fn check_state_machine() {
    //     let data = SAMPLE;
    //     let result = State::new(data.bytes()).process();

    //     println!("State machine says: {:?}", result);

    //     panic!();
    // }
}
