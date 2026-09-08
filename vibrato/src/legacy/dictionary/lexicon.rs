pub(crate) mod feature;
pub(crate) mod map;
pub(crate) mod param;

use bincode::{Decode, Encode};

use crate::legacy::dictionary::LexType;
use crate::legacy::dictionary::lexicon::feature::WordFeatures;
use crate::legacy::dictionary::lexicon::map::WordMap;
use crate::legacy::dictionary::lexicon::param::WordParams;

/// Lexicon of words.
#[derive(Decode, Encode)]
pub struct Lexicon {
    pub(crate) map: WordMap,
    pub(crate) params: WordParams,
    pub(crate) features: WordFeatures,
    pub(crate) lex_type: LexType,
}
