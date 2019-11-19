#[derive(Debug)]
pub(super) enum NodeKind {
    Key(Vec<u8>),
    ScalarPlain(Vec<u8>),
}
