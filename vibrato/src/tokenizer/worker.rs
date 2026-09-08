//! Provider of a routine for tokenization.
use crate::dictionary::connector::ConnectorView;
use crate::dictionary::mapper::{ConnIdCounter, ConnIdProbs};
use crate::dictionary::{ConnectorKindRef, DictionaryInnerRef};
use crate::sentence::Sentence;
use crate::token::{NbestTokenIter, Token, TokenIter};
use crate::tokenizer::Tokenizer;
use crate::tokenizer::lattice::{Lattice, LatticeKind, Node};
use crate::tokenizer::nbest_generator::NbestGenerator;

/// Provider of a routine for tokenization.
///
/// It holds the internal data structures used in tokenization,
/// which can be reused to avoid unnecessary memory reallocation.
pub struct Worker {
    one_best_ready: bool,
    pub(crate) tokenizer: Tokenizer,
    pub(crate) sent: Sentence,
    pub(crate) lattice: LatticeKind,
    pub(crate) top_nodes: Vec<(usize, Node)>,
    pub(crate) counter: Option<ConnIdCounter>,
    pub(crate) nbest_paths: Vec<(Vec<*const Node>, i32)>,
}

impl Worker {
    /// Creates a new instance.
    pub(crate) fn new(tokenizer: Tokenizer) -> Self {
        Self {
            one_best_ready: false,
            tokenizer,
            sent: Sentence::new(),
            lattice: LatticeKind::For1Best(Lattice::default()),
            top_nodes: vec![],
            counter: None,
            nbest_paths: Vec::with_capacity(0),
        }
    }

    /// Resets the input sentence to be tokenized.
    pub fn reset_sentence<S>(&mut self, input: S)
    where
        S: AsRef<str>,
    {
        self.nbest_paths.clear();
        self.one_best_ready = false;
        self.sent.clear();
        self.top_nodes.clear();
        let input = input.as_ref();
        if !input.is_empty() {
            self.sent.set_sentence(input);
            match self.tokenizer.dictionary() {
                DictionaryInnerRef::Archived(dict) => {
                    self.sent.compile_archived(dict.char_prop());
                }
                DictionaryInnerRef::Owned(dict) => {
                    self.sent.compile(dict.char_prop());
                }
            }
        }
    }

    /// Tokenizes the input sentence set in `state`,
    /// returning the result through `state`.
    pub fn tokenize(&mut self) {
        self.one_best_ready = false;
        self.top_nodes.clear();
        self.nbest_paths.clear();
        if self.sent.chars().is_empty() {
            return;
        }
        let lattice_1best = self.lattice.prepare_for_1best(self.sent.len_char());

        self.tokenizer.build_lattice(&self.sent, lattice_1best);
        lattice_1best.append_top_nodes(&mut self.top_nodes);
        self.one_best_ready = true;
    }

    /// Tokenizes the sentence and stores the top N-best results internally.
    ///
    /// After calling this, the results can be accessed via `num_nbest_paths()`,
    /// `path_cost(path_idx)`, and `nbest_token_iter(path_idx)`.
    pub fn tokenize_nbest(&mut self, n: usize) {
        self.one_best_ready = false;
        self.top_nodes.clear();
        self.nbest_paths.clear();
        if self.sent.chars().is_empty() {
            return;
        }
        let lattice_nbest = self.lattice.prepare_for_nbest(self.sent.len_char());

        self.tokenizer
            .build_lattice_nbest(&self.sent, lattice_nbest);

        let dict_ref = self.tokenizer.dictionary();
        let connector_ref = dict_ref.connector();

        let generator = match connector_ref {
            ConnectorKindRef::Archived(connector) => {
                NbestGenerator::new(lattice_nbest, connector, dict_ref)
            }
            ConnectorKindRef::Owned(connector) => {
                NbestGenerator::new(lattice_nbest, connector, dict_ref)
            }
        };
        self.nbest_paths = generator.take(n).collect();
    }

    /// Gets the number of resultant tokens.
    #[inline(always)]
    pub fn num_tokens(&self) -> usize {
        self.top_nodes.len()
    }

    /// Gets the `i`-th resultant token.
    #[inline(always)]
    pub fn token<'w>(&'w self, i: usize) -> Token<'w> {
        let index = self.num_tokens() - i - 1;
        Token::new(self, index)
    }

    /// Creates an iterator of resultant tokens.
    #[inline(always)]
    pub fn token_iter<'w>(&'w self) -> TokenIter<'w> {
        TokenIter::new(self)
    }

    /// Returns an iterator over the tokens in the N-best path at `path_idx`.
    pub fn nbest_token_iter(&self, path_idx: usize) -> Option<NbestTokenIter<'_>> {
        if path_idx < self.nbest_paths.len() {
            Some(NbestTokenIter::new(self, path_idx))
        } else {
            None
        }
    }

    /// Initializes a counter to compute occurrence probabilities of connection ids.
    pub fn init_connid_counter(&mut self) {
        let (num_left, num_right) = match self.tokenizer.dictionary() {
            DictionaryInnerRef::Archived(dict) => {
                (dict.connector().num_left(), dict.connector().num_right())
            }
            DictionaryInnerRef::Owned(dict) => {
                (dict.connector().num_left(), dict.connector().num_right())
            }
        };
        self.counter = Some(ConnIdCounter::new(num_left, num_right));
    }

    /// Updates frequencies of connection ids at the last tokenization.
    ///
    /// # Panics
    ///
    /// It will panic when [`Self::init_connid_counter()`] has never been called.
    pub fn update_connid_counts(&mut self) {
        match &self.lattice {
            LatticeKind::For1Best(lattice) => {
                lattice.add_connid_counts(self.counter.as_mut().unwrap())
            }
            LatticeKind::ForNBest(lattice_nbest) => {
                lattice_nbest.add_connid_counts(self.counter.as_mut().unwrap())
            }
        }
    }

    /// Computes the computed occurrence probabilities of connection ids,
    /// returning those for left- and right-ids.
    ///
    /// # Panics
    ///
    /// It will panic when [`Self::init_connid_counter()`] has never been called.
    pub fn compute_connid_probs(&self) -> (ConnIdProbs, ConnIdProbs) {
        self.counter.as_ref().unwrap().compute_probs()
    }

    /// Returns the number of N-best paths found.
    pub fn num_nbest_paths(&self) -> usize {
        self.nbest_paths.len()
    }

    /// Returns the total cost of the path at `path_idx`.
    pub fn path_cost(&self, path_idx: usize) -> Option<i32> {
        self.nbest_paths.get(path_idx).map(|(_, cost)| *cost)
    }
}

/// A candidate on a complete path through the tokenized sentence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LatticeCandidate {
    /// Character boundary including any skipped leading spaces.
    pub start_node: usize,
    /// Character span of the surface, excluding skipped spaces.
    pub range_char: std::ops::Range<usize>,
    /// Byte span of the surface, excluding skipped spaces.
    pub range_byte: std::ops::Range<usize>,
    /// Dictionary feature string.
    pub feature: String,
    /// Identifier of the dictionary entry.
    pub word_idx: crate::dictionary::word_idx::WordIdx,
    /// Left connection identifier.
    pub left_id: u16,
    /// Right connection identifier.
    pub right_id: u16,
    /// Dictionary word cost.
    pub word_cost: i16,
    /// Minimum cost from BOS through this candidate, including its word cost.
    pub cost: i64,
    /// Additional sentence cost when the path must pass through this candidate.
    pub delta: i64,
}

/// Owned lattice candidates and the selected best path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LatticeSnapshot {
    /// Candidates lying on at least one complete path from BOS to EOS.
    pub nodes: Vec<LatticeCandidate>,
    /// Indices into `nodes`, in the selected path's order.
    pub best_path: Vec<usize>,
    /// Minimum sentence cost including the transition to EOS.
    pub total_cost: i64,
}

impl Worker {
    /// Returns the minimum sentence cost after non-empty 1-best tokenization.
    pub fn total_cost(&self) -> Option<i64> {
        if !self.one_best_ready {
            return None;
        }
        match &self.lattice {
            LatticeKind::For1Best(lattice) => lattice.eos().map(|node| i64::from(node.min_cost)),
            LatticeKind::ForNBest(_) => None,
        }
    }

    /// Returns all candidates after `tokenize`, including their minimum path-cost differences.
    ///
    /// Returns `None` before tokenization or after N-best tokenization. The snapshot owns
    /// its data and remains valid when the worker is reused. BOS and EOS are omitted.
    pub fn lattice_snapshot(&self) -> Option<LatticeSnapshot> {
        use crate::dictionary::connector::ConnectorCost;
        if !self.one_best_ready {
            return None;
        }
        let LatticeKind::For1Best(lattice) = &self.lattice else {
            return None;
        };
        let eos = lattice.eos()?;
        let dict = self.tokenizer.dictionary();
        let connector = dict.connector();
        let transition = |right, left| -> i64 {
            match &connector {
                ConnectorKindRef::Owned(c) => i64::from(c.cost(right, left)),
                ConnectorKindRef::Archived(c) => i64::from(c.cost(right, left)),
            }
        };
        let mut nodes: Vec<_> = lattice
            .nodes()
            .map(|(end, node)| {
                let feature = match dict {
                    DictionaryInnerRef::Owned(d) => d.word_feature(node.word_idx()),
                    DictionaryInnerRef::Archived(d) => d.word_feature(node.word_idx()),
                };
                let param = match dict {
                    DictionaryInnerRef::Owned(d) => d.word_param(node.word_idx()),
                    DictionaryInnerRef::Archived(d) => d.word_param(node.word_idx()),
                };
                LatticeCandidate {
                    start_node: node.start_node,
                    range_char: node.start_word..end,
                    range_byte: self.sent.byte_position(node.start_word)
                        ..self.sent.byte_position(end),
                    feature: feature.to_owned(),
                    word_idx: node.word_idx(),
                    left_id: node.left_id,
                    right_id: node.right_id,
                    word_cost: param.word_cost,
                    cost: i64::from(node.min_cost),
                    delta: 0,
                }
            })
            .collect();
        let mut begin = vec![Vec::new(); self.sent.len_char() + 1];
        for (i, node) in nodes.iter().enumerate() {
            begin[node.start_node].push(i);
        }
        let mut backward = vec![i64::MAX; nodes.len()];
        let total_cost = i64::from(eos.min_cost);
        for (i, node) in nodes.iter().enumerate().rev() {
            let mut rest = if node.range_char.end == eos.start_node {
                transition(node.right_id, 0)
            } else {
                i64::MAX
            };
            for &j in &begin[node.range_char.end] {
                if backward[j] != i64::MAX {
                    rest = rest.min(
                        backward[j]
                            + i64::from(nodes[j].word_cost)
                            + transition(node.right_id, nodes[j].left_id),
                    );
                }
            }
            backward[i] = rest;
        }
        let mut i = 0;
        nodes.retain_mut(|node| {
            let rest = backward[i];
            i += 1;
            if rest == i64::MAX {
                return false;
            }
            node.delta = node.cost + rest - total_cost;
            true
        });
        let best_path = self
            .token_iter()
            .map(|token| {
                nodes
                    .iter()
                    .position(|node| {
                        node.word_idx == token.word_idx() && node.range_byte == token.range_byte()
                    })
                    .expect("best path is present in the lattice")
            })
            .collect();
        Some(LatticeSnapshot {
            nodes,
            best_path,
            total_cost,
        })
    }
}
