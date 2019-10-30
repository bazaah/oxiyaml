trait Shared {
    fn functionality(&self) -> usize;
}

#[derive(Default, Debug)]
struct Waiting {
    duration: usize,
    shared: usize
}

impl Waiting {
    fn to_filling(self) -> Filling {
        Filling {
            rate: 1,
            shared: self.shared
        }
    } 
}

impl Shared for Waiting {
    fn functionality(&self) -> usize {
        self.shared
    }
}

#[derive(Default, Debug)]
struct Filling {
    rate: usize,
    shared: usize,
}

impl Filling {
    fn to_done(self) -> Done {
        Done {
            shared: self.shared
        }
    }
}

impl Shared for Filling {
    fn functionality(&self) -> usize {
        self.shared
    }
}

#[derive(Default, Debug)]
struct Done {
    shared: usize
}

impl Shared for Done {
    fn functionality(&self) -> usize {
        self.shared
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transistion() {
        let state = Waiting { duration: 0, shared: 42 };
        assert!(state.functionality() == 42);

        let state = state.to_filling();

        assert!(state.functionality() == 42);

        let state = state.to_done();

        assert!(state.functionality() == 42);
    }
}
