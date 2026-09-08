use rkyv::{Archive, Deserialize, Serialize};

#[derive(Default, Archive, Serialize, Deserialize)]
pub struct WordFeatures {
    features: Vec<String>,
}

impl WordFeatures {
    pub fn new<I, S>(features: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        Self {
            features: features
                .into_iter()
                .map(|s| s.as_ref().to_string())
                .collect(),
        }
    }

    #[inline(always)]
    pub fn get(&self, word_id: usize) -> &str {
        &self.features[word_id]
    }
}

impl ArchivedWordFeatures {
    #[inline(always)]
    pub fn get(&self, word_id: usize) -> &str {
        &self.features[word_id]
    }
}

#[cfg(feature = "legacy")]
impl From<crate::legacy::dictionary::lexicon::feature::WordFeatures> for WordFeatures {
    fn from(old: crate::legacy::dictionary::lexicon::feature::WordFeatures) -> Self {
        Self {
            features: old.features,
        }
    }
}
