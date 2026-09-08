use bincode::{Decode, Encode};

/// Mapper for connection ids.
#[derive(Decode, Encode)]
pub struct ConnIdMapper {
    pub(crate) left: Vec<u16>,
    pub(crate) right: Vec<u16>,
}
