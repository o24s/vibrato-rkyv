pub mod scorer;

use bincode::{Decode, Encode};

use crate::legacy::dictionary::connector::raw_connector::scorer::{Scorer, U31x8};

#[derive(Decode, Encode)]
pub struct RawConnector {
    pub(crate) right_feat_ids: Vec<U31x8>,
    pub(crate) left_feat_ids: Vec<U31x8>,
    pub(crate) feat_template_size: usize,
    pub(crate) scorer: Scorer,
}
