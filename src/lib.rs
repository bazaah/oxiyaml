#![allow(dead_code)]

mod machine;
mod node;
mod scanner;
mod states;

pub(self) use crate::{machine::StateMachine, scanner::*, states::*, node::NodeKind};

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

    fn next_output(&mut self) -> Result<NodeKind, String> {} 
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

    fn process(self) -> Result<String, String> {
        let mut check = 0;
        let mut bind = self;

        loop {
            match bind {
                Self::Done(dn) => return Ok(dn.done()),
                Self::Failure(fail) => return Err(fail.error()),
                _ if check > 100 => return Err(format!("Something is wrong in step")),
                _ => {
                    check += 1;
                    bind = bind.step()
                }
            }
        }
    }

    fn step(self) -> Self {
        match self {
            Self::Start(mut st) => match st.cycle() {
                Marker::LineStart => Self::LineStart(st.into()),
                Marker::Done => Self::Done(st.into()),
                _ => Self::Failure((format!("Invalid transition"), st).into()),
            },
            Self::WhiteSpace(mut st) => match st.cycle() {
                Marker::LineEnd => Self::LineEnd(st.into()),
                Marker::Ignore => Self::Ignore(st.into()),
                Marker::Done => Self::Done(st.into()),
                _ => Self::Failure((format!("Invalid transition"), st).into()),
            },
            Self::Ignore(mut st) => match st.cycle() {
                Marker::LineEnd => Self::LineEnd(st.into()),
                Marker::WhiteSpace => Self::WhiteSpace(st.into()),
                Marker::Done => Self::Done(st.into()),
                Marker::Failure => Self::Failure((format!("State violation (Ignore)"), st).into()),
                _ => Self::Failure((format!("Invalid transition"), st).into()),
            },
            Self::LineStart(mut st) => match st.cycle() {
                Marker::LineEnd => Self::LineEnd(st.into()),
                Marker::Ignore => Self::Ignore(st.into()),
                Marker::Done => Self::Done(st.into()),
                _ => Self::Failure((format!("Invalid transition"), st).into()),
            },
            Self::LineEnd(mut st) => match st.cycle() {
                Marker::LineStart => Self::LineStart(st.into()),
                Marker::Done => Self::Done(st.into()),
                Marker::Failure => Self::Failure((format!("State violation (LineEnd)"), st).into()),
                _ => Self::Failure((format!("Invalid transition"), st).into()),
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

    #[test]
    fn check_state_machine() {
        let data = SAMPLE;
        let result = State::new(data.bytes()).process();

        println!("State machine says: {:?}", result);

        panic!();
    }
}
