pub(crate) mod dual_connector;
pub(crate) mod matrix_connector;
pub(crate) mod raw_connector;

use bincode::{Decode, Encode};

pub use crate::legacy::dictionary::connector::dual_connector::DualConnector;
pub use crate::legacy::dictionary::connector::matrix_connector::MatrixConnector;
pub use crate::legacy::dictionary::connector::raw_connector::RawConnector;

#[derive(Decode, Encode)]
pub enum ConnectorWrapper {
    Matrix(MatrixConnector),
    Raw(RawConnector),
    Dual(DualConnector),
}
