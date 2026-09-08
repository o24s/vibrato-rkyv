use bincode::{Decode, Encode};

#[derive(Default, Decode, Encode)]
pub struct WordFeatures {
    pub(crate) features: Vec<String>,
}
