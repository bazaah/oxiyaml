#[derive(Debug, PartialEq)]
struct StateMachine(State);

impl StateMachine {
    fn new() -> Self {
        Default::default()
    }

    fn filling(&mut self) {
        self.0 = match self.0 {
            State::Waiting { .. } => State::Filling{ duration: 0 },
            _ => panic!("Invalid state transition")
        }
    }
}

impl Default for StateMachine {
    fn default() -> Self {
        StateMachine(State::Waiting{ wait_time: 0 })
    }
}

#[derive(Debug, PartialEq)]
enum State {
    Waiting { wait_time: usize },
    Filling { duration: usize },
    Done
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_filling() {
        let mut state = StateMachine::new();
        state.filling();

        assert_eq!(state, StateMachine(State::Filling{ duration: 0 }))
    }
}
