use bincode::{Decode, Encode};

/// Matrix of connection costs.
#[derive(Decode, Encode)]
pub struct MatrixConnector {
    pub(crate) data: Vec<i16>,
    pub(crate) num_right: usize,
    pub(crate) num_left: usize,
}
