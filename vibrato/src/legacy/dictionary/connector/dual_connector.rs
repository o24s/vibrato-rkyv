use bincode::{Decode, Encode};

use crate::legacy::dictionary::connector::MatrixConnector;
use crate::legacy::dictionary::connector::raw_connector::scorer::{Scorer, U31x8};

#[derive(Decode, Encode)]
pub struct DualConnector {
    pub(crate) matrix_connector: MatrixConnector,
    pub(crate) right_conn_id_map: Vec<u16>,
    pub(crate) left_conn_id_map: Vec<u16>,
    pub(crate) right_feat_ids: Vec<U31x8>,
    pub(crate) left_feat_ids: Vec<U31x8>,
    pub(crate) raw_scorer: Scorer,
}
