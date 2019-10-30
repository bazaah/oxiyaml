struct Machine<S = Waiting> {
    shared: usize,
    state: S,
}

impl Default for Machine {
    fn default() -> Self {
        Self {
            shared: 0,
            state: Waiting::default(),
        }
    }
}

impl From<Machine<Waiting>> for Machine<Filling> {
    fn from(m: Machine<Waiting>) -> Self {
        Self {
            shared: m.shared,
            state: Filling { rate: 1 },
        }
    }
}

impl From<Machine<Filling>> for Machine<Done> {
    fn from(m: Machine<Filling>) -> Self {
        Self {
            shared: m.shared,
            state: Done,
        }
    }
}

#[derive(Default)]
struct Waiting {
    duration: usize,
}

struct Filling {
    rate: usize,
}

struct Done;
