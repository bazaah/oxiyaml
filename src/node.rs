use super::Error;

#[derive(Debug)]
pub(super) enum NodeKind {
    Key(String),
    ScalarPlain(String),
    Failure(Error),
    Done,
}
