use crate::dictionary::SystemDictionaryBuilder;
use crate::{Dictionary, Tokenizer};

const LEX_CSV: &str = include_str!("./resources/lex.csv");
const USER_CSV: &str = include_str!("./resources/user.csv");
const MATRIX_DEF: &str = include_str!("./resources/matrix.def");
const CHAR_DEF: &str = include_str!("./resources/char.def");
const UNK_DEF: &str = include_str!("./resources/unk.def");

#[track_caller]
fn build_test_dictionary(
    lexicon_csv: &[u8],
    matrix_def: &[u8],
    char_def: &[u8],
    unk_def: &[u8],
) -> Dictionary {
    let dict_inner =
        SystemDictionaryBuilder::from_readers(lexicon_csv, matrix_def, char_def, unk_def).unwrap();

    Dictionary::from_inner(dict_inner)
}

#[test]
fn test_tokenize_tokyo() {
    let dict = build_test_dictionary(
        LEX_CSV.as_bytes(),
        MATRIX_DEF.as_bytes(),
        CHAR_DEF.as_bytes(),
        UNK_DEF.as_bytes(),
    );

    let tokenizer = Tokenizer::new(dict);
    let mut worker = tokenizer.new_worker();
    worker.reset_sentence("東京都");
    worker.tokenize();
    assert_eq!(worker.num_tokens(), 1);

    {
        let t = worker.token(0);
        assert_eq!(t.surface(), "東京都");
        assert_eq!(t.range_char(), 0..3);
        assert_eq!(t.range_byte(), 0..9);
        assert_eq!(
            t.feature(),
            "東京都,名詞,固有名詞,地名,一般,*,*,トウキョウト,東京都,*,B,5/9,*,5/9,*"
        );
    }

    //   c=0      c=5320       c=0
    //  [BOS] -- [東京都] -- [EOS]
    //     r=0  l=6   r=8  l=0
    //      c=-79
    assert_eq!(worker.token(0).total_cost(), -79 + 5320);
}

#[test]
fn test_tokenize_kyotokyo() {
    let dict = build_test_dictionary(
        LEX_CSV.as_bytes(),
        MATRIX_DEF.as_bytes(),
        CHAR_DEF.as_bytes(),
        UNK_DEF.as_bytes(),
    );

    let tokenizer = Tokenizer::new(dict);
    let mut worker = tokenizer.new_worker();
    worker.reset_sentence("京都東京都京都");
    worker.tokenize();
    assert_eq!(worker.num_tokens(), 3);

    {
        let t = worker.token(0);
        assert_eq!(t.surface(), "京都");
        assert_eq!(t.range_char(), 0..2);
        assert_eq!(t.range_byte(), 0..6);
        assert_eq!(
            t.feature(),
            "京都,名詞,固有名詞,地名,一般,*,*,キョウト,京都,*,A,*,*,*,1/5"
        );
    }
    {
        let t = worker.token(1);
        assert_eq!(t.surface(), "東京都");
        assert_eq!(t.range_char(), 2..5);
        assert_eq!(t.range_byte(), 6..15);
        assert_eq!(
            t.feature(),
            "東京都,名詞,固有名詞,地名,一般,*,*,トウキョウト,東京都,*,B,5/9,*,5/9,*"
        );
    }
    {
        let t = worker.token(2);
        assert_eq!(t.surface(), "京都");
        assert_eq!(t.range_char(), 5..7);
        assert_eq!(t.range_byte(), 15..21);
        assert_eq!(
            t.feature(),
            "京都,名詞,固有名詞,地名,一般,*,*,キョウト,京都,*,A,*,*,*,1/5"
        );
    }

    //   c=0     c=5293    c=5320    c=5293    c=0
    //  [BOS] -- [京都] -- [東京都] -- [京都] -- [EOS]
    //     r=0  l=6  r=6  l=6  r=8  l=6  r=6  l=0
    //      c=-79     c=569     c=-352
    assert_eq!(worker.token(0).total_cost(), -79 + 5293);
    assert_eq!(
        worker.token(1).total_cost(),
        worker.token(0).total_cost() + 569 + 5320
    );
    assert_eq!(
        worker.token(2).total_cost(),
        worker.token(1).total_cost() - 352 + 5293
    );
}

#[test]
fn test_tokenize_kyotokyo_with_user() {
    let dict = {
        let lexicon_csv = LEX_CSV.as_bytes();
        let matrix_def = MATRIX_DEF.as_bytes();
        let char_def = CHAR_DEF.as_bytes();
        let unk_def = UNK_DEF.as_bytes();
        let dict_inner =
            SystemDictionaryBuilder::from_readers(lexicon_csv, matrix_def, char_def, unk_def)
                .unwrap();

        let dict_inner = dict_inner
            .reset_user_lexicon_from_reader(Some(USER_CSV.as_bytes()))
            .unwrap();

        Dictionary::from_inner(dict_inner)
    };

    let tokenizer = Tokenizer::new(dict);
    let mut worker = tokenizer.new_worker();
    worker.reset_sentence("京都東京都京都");
    worker.tokenize();
    assert_eq!(worker.num_tokens(), 2);

    {
        let t = worker.token(0);
        assert_eq!(t.surface(), "京都東京都");
        assert_eq!(t.range_char(), 0..5);
        assert_eq!(t.range_byte(), 0..15);
        assert_eq!(t.feature(), "カスタム名詞");
    }
    {
        let t = worker.token(1);
        assert_eq!(t.surface(), "京都");
        assert_eq!(t.range_char(), 5..7);
        assert_eq!(t.range_byte(), 15..21);
        assert_eq!(
            t.feature(),
            "京都,名詞,固有名詞,地名,一般,*,*,キョウト,京都,*,A,*,*,*,1/5"
        );
    }

    //   c=0      c=-1000      c=5293    c=0
    //  [BOS] -- [京都東京都] -- [京都] -- [EOS]
    //     r=0  l=6      r=8  l=6  r=6  l=0
    //      c=-79         c=-352
    assert_eq!(worker.token(0).total_cost(), -79 - 1000);
    assert_eq!(
        worker.token(1).total_cost(),
        worker.token(0).total_cost() - 352 + 5293
    );
}

#[test]
fn test_tokenize_tokyoto_with_space() {
    let dict = build_test_dictionary(
        LEX_CSV.as_bytes(),
        MATRIX_DEF.as_bytes(),
        CHAR_DEF.as_bytes(),
        UNK_DEF.as_bytes(),
    );

    let tokenizer = Tokenizer::new(dict);
    let mut worker = tokenizer.new_worker();
    worker.reset_sentence("東京 都");
    worker.tokenize();
    assert_eq!(worker.num_tokens(), 3);

    {
        let t = worker.token(0);
        assert_eq!(t.surface(), "東京");
        assert_eq!(t.range_char(), 0..2);
        assert_eq!(t.range_byte(), 0..6);
        assert_eq!(
            t.feature(),
            "東京,名詞,固有名詞,地名,一般,*,*,トウキョウ,東京,*,A,*,*,*,*"
        );
    }
    {
        let t = worker.token(1);
        assert_eq!(t.surface(), " ");
        assert_eq!(t.range_char(), 2..3);
        assert_eq!(t.range_byte(), 6..7);
        assert_eq!(t.feature(), " ,空白,*,*,*,*,*, , ,*,A,*,*,*,*");
    }
    {
        let t = worker.token(2);
        assert_eq!(t.surface(), "都");
        assert_eq!(t.range_char(), 3..4);
        assert_eq!(t.range_byte(), 7..10);
        assert_eq!(t.feature(), "都,名詞,普通名詞,一般,*,*,*,ト,都,*,A,*,*,*,*");
    }

    //   c=0     c=2816 c=-20000 c=2914   c=0
    //  [BOS] -- [東京] -- [ ] -- [都] -- [EOS]
    //     r=0  l=6 r=6 l=8 r=8 l=8 r=8 l=0
    //      c=-79    c=-390  c=1134  c=-522
    assert_eq!(worker.token(0).total_cost(), -79 + 2816);
    assert_eq!(
        worker.token(1).total_cost(),
        worker.token(0).total_cost() - 390 - 20000
    );
    assert_eq!(
        worker.token(2).total_cost(),
        worker.token(1).total_cost() + 1134 + 2914
    );
}

#[test]
fn test_tokenize_tokyoto_with_space_ignored() {
    let dict = build_test_dictionary(
        LEX_CSV.as_bytes(),
        MATRIX_DEF.as_bytes(),
        CHAR_DEF.as_bytes(),
        UNK_DEF.as_bytes(),
    );

    let tokenizer = Tokenizer::new(dict).ignore_space(true).unwrap();
    let mut worker = tokenizer.new_worker();
    worker.reset_sentence("東京 都");
    worker.tokenize();
    assert_eq!(worker.num_tokens(), 2);

    {
        let t = worker.token(0);
        assert_eq!(t.surface(), "東京");
        assert_eq!(t.range_char(), 0..2);
        assert_eq!(t.range_byte(), 0..6);
        assert_eq!(
            t.feature(),
            "東京,名詞,固有名詞,地名,一般,*,*,トウキョウ,東京,*,A,*,*,*,*"
        );
    }
    {
        let t = worker.token(1);
        assert_eq!(t.surface(), "都");
        assert_eq!(t.range_char(), 3..4);
        assert_eq!(t.range_byte(), 7..10);
        assert_eq!(t.feature(), "都,名詞,普通名詞,一般,*,*,*,ト,都,*,A,*,*,*,*");
    }

    //   c=0     c=2816   c=2914   c=0
    //  [BOS] -- [東京] -- [都] -- [EOS]
    //     r=0  l=6 r=6  l=8 r=8 l=0
    //      c=-79    c=-390  c=-522
    assert_eq!(worker.token(0).total_cost(), -79 + 2816);
    assert_eq!(
        worker.token(1).total_cost(),
        worker.token(0).total_cost() - 390 + 2914
    );
}

#[test]
fn test_tokenize_tokyoto_with_spaces_ignored() {
    let dict = build_test_dictionary(
        LEX_CSV.as_bytes(),
        MATRIX_DEF.as_bytes(),
        CHAR_DEF.as_bytes(),
        UNK_DEF.as_bytes(),
    );

    let tokenizer = Tokenizer::new(dict).ignore_space(true).unwrap();
    let mut worker = tokenizer.new_worker();
    worker.reset_sentence("東京   都");
    worker.tokenize();
    assert_eq!(worker.num_tokens(), 2);

    {
        let t = worker.token(0);
        assert_eq!(t.surface(), "東京");
        assert_eq!(t.range_char(), 0..2);
        assert_eq!(t.range_byte(), 0..6);
        assert_eq!(
            t.feature(),
            "東京,名詞,固有名詞,地名,一般,*,*,トウキョウ,東京,*,A,*,*,*,*"
        );
    }
    {
        let t = worker.token(1);
        assert_eq!(t.surface(), "都");
        assert_eq!(t.range_char(), 5..6);
        assert_eq!(t.range_byte(), 9..12);
        assert_eq!(t.feature(), "都,名詞,普通名詞,一般,*,*,*,ト,都,*,A,*,*,*,*");
    }

    //   c=0     c=2816   c=2914   c=0
    //  [BOS] -- [東京] -- [都] -- [EOS]
    //     r=0  l=6 r=6  l=8 r=8 l=0
    //      c=-79    c=-390  c=-522
    assert_eq!(worker.token(0).total_cost(), -79 + 2816);
    assert_eq!(
        worker.token(1).total_cost(),
        worker.token(0).total_cost() - 390 + 2914
    );
}

#[test]
fn test_tokenize_tokyoto_startswith_spaces_ignored() {
    let dict = build_test_dictionary(
        LEX_CSV.as_bytes(),
        MATRIX_DEF.as_bytes(),
        CHAR_DEF.as_bytes(),
        UNK_DEF.as_bytes(),
    );

    let tokenizer = Tokenizer::new(dict).ignore_space(true).unwrap();
    let mut worker = tokenizer.new_worker();
    worker.reset_sentence("   東京都");
    worker.tokenize();
    assert_eq!(worker.num_tokens(), 1);

    {
        let t = worker.token(0);
        assert_eq!(t.surface(), "東京都");
        assert_eq!(t.range_char(), 3..6);
        assert_eq!(t.range_byte(), 3..12);
        assert_eq!(
            t.feature(),
            "東京都,名詞,固有名詞,地名,一般,*,*,トウキョウト,東京都,*,B,5/9,*,5/9,*"
        );
    }

    //   c=0      c=5320       c=0
    //  [BOS] -- [東京都] -- [EOS]
    //     r=0  l=6   r=8  l=0
    //      c=-79
    assert_eq!(worker.token(0).total_cost(), -79 + 5320);
}

#[test]
fn test_tokenize_tokyoto_endswith_spaces_ignored() {
    let dict = build_test_dictionary(
        LEX_CSV.as_bytes(),
        MATRIX_DEF.as_bytes(),
        CHAR_DEF.as_bytes(),
        UNK_DEF.as_bytes(),
    );

    let tokenizer = Tokenizer::new(dict).ignore_space(true).unwrap();
    let mut worker = tokenizer.new_worker();
    worker.reset_sentence("東京都   ");
    worker.tokenize();
    assert_eq!(worker.num_tokens(), 1);

    {
        let t = worker.token(0);
        assert_eq!(t.surface(), "東京都");
        assert_eq!(t.range_char(), 0..3);
        assert_eq!(t.range_byte(), 0..9);
        assert_eq!(
            t.feature(),
            "東京都,名詞,固有名詞,地名,一般,*,*,トウキョウト,東京都,*,B,5/9,*,5/9,*"
        );
    }

    //   c=0      c=5320       c=0
    //  [BOS] -- [東京都] -- [EOS]
    //     r=0  l=6   r=8  l=0
    //      c=-79
    assert_eq!(worker.token(0).total_cost(), -79 + 5320);
}

#[test]
fn test_tokenize_kampersanda() {
    let dict = build_test_dictionary(
        LEX_CSV.as_bytes(),
        MATRIX_DEF.as_bytes(),
        CHAR_DEF.as_bytes(),
        UNK_DEF.as_bytes(),
    );

    let tokenizer = Tokenizer::new(dict);
    let mut worker = tokenizer.new_worker();
    worker.reset_sentence("kampersanda");
    worker.tokenize();
    assert_eq!(worker.num_tokens(), 1);

    {
        let t = worker.token(0);
        assert_eq!(t.surface(), "kampersanda");
        assert_eq!(t.range_char(), 0..11);
        assert_eq!(t.range_byte(), 0..11);
        assert_eq!(t.feature(), "名詞,普通名詞,一般,*,*,*");
    }

    //   c=0        c=11633         c=0
    //  [BOS] -- [kampersanda] -- [EOS]
    //     r=0  l=7         r=7  l=0
    //      c=887
    assert_eq!(worker.token(0).total_cost(), 887 + 11633);
}

#[test]
fn test_tokenize_kampersanda_with_user() {
    let dict = {
        let lexicon_csv = LEX_CSV.as_bytes();
        let matrix_def = MATRIX_DEF.as_bytes();
        let char_def = CHAR_DEF.as_bytes();
        let unk_def = UNK_DEF.as_bytes();
        let dict_inner =
            SystemDictionaryBuilder::from_readers(lexicon_csv, matrix_def, char_def, unk_def)
                .unwrap();

        let dict_inner = dict_inner
            .reset_user_lexicon_from_reader(Some(USER_CSV.as_bytes()))
            .unwrap();

        Dictionary::from_inner(dict_inner)
    };

    let tokenizer = Tokenizer::new(dict);
    let mut worker = tokenizer.new_worker();
    worker.reset_sentence("kampersanda");
    worker.tokenize();
    assert_eq!(worker.num_tokens(), 1);

    {
        let t = worker.token(0);
        assert_eq!(t.surface(), "kampersanda");
        assert_eq!(t.range_char(), 0..11);
        assert_eq!(t.range_byte(), 0..11);
        assert_eq!(t.feature(), "カスタム名詞");
    }

    //   c=0        c=-2000        c=0
    //  [BOS] -- [kampersanda] -- [EOS]
    //     r=0  l=7         r=7  l=0
    //      c=887
    assert_eq!(worker.token(0).total_cost(), 887 - 2000);
}

#[test]
fn test_tokenize_kampersanda_with_max_grouping() {
    let dict = build_test_dictionary(
        LEX_CSV.as_bytes(),
        MATRIX_DEF.as_bytes(),
        CHAR_DEF.as_bytes(),
        UNK_DEF.as_bytes(),
    );

    let tokenizer = Tokenizer::new(dict)
        .ignore_space(true)
        .unwrap()
        .max_grouping_len(9);
    let mut worker = tokenizer.new_worker();
    worker.reset_sentence("kampersanda");
    worker.tokenize();
    assert_eq!(worker.num_tokens(), 2);

    {
        let t = worker.token(0);
        assert_eq!(t.surface(), "k");
        assert_eq!(t.range_char(), 0..1);
        assert_eq!(t.range_byte(), 0..1);
        assert_eq!(t.feature(), "名詞,普通名詞,一般,*,*,*");
    }
    {
        let t = worker.token(1);
        assert_eq!(t.surface(), "ampersanda");
        assert_eq!(t.range_char(), 1..11);
        assert_eq!(t.range_byte(), 1..11);
        assert_eq!(t.feature(), "名詞,普通名詞,一般,*,*,*");
    }

    //   c=0   c=11633    c=11633        c=0
    //  [BOS] -- [k] -- [ampersanda] -- [EOS]
    //     r=0 l=7 r=7 l=7        r=7  l=0
    //      c=887   c=2341
    assert_eq!(worker.token(0).total_cost(), 887 + 11633);
    assert_eq!(
        worker.token(1).total_cost(),
        worker.token(0).total_cost() + 2341 + 11633
    );
}

#[test]
fn test_tokenize_tokyoken() {
    let dict = build_test_dictionary(
        LEX_CSV.as_bytes(),
        MATRIX_DEF.as_bytes(),
        CHAR_DEF.as_bytes(),
        UNK_DEF.as_bytes(),
    );

    let tokenizer = Tokenizer::new(dict);
    let mut worker = tokenizer.new_worker();
    worker.reset_sentence("東京県に行く");
    worker.tokenize();
    assert_eq!(worker.num_tokens(), 4);
}

/// This test is to check if the category order in char.def is preserved.
#[test]
fn test_tokenize_kanjinumeric() {
    let dict = build_test_dictionary(
        LEX_CSV.as_bytes(),
        MATRIX_DEF.as_bytes(),
        CHAR_DEF.as_bytes(),
        UNK_DEF.as_bytes(),
    );

    let tokenizer = Tokenizer::new(dict);
    let mut worker = tokenizer.new_worker();
    worker.reset_sentence("一橋大学大学院");
    worker.tokenize();
    assert_eq!(worker.num_tokens(), 1);

    {
        let t = worker.token(0);
        assert_eq!(t.surface(), "一橋大学大学院");
        assert_eq!(t.range_char(), 0..7);
        assert_eq!(t.range_byte(), 0..21);
        assert_eq!(t.feature(), "名詞,数,*,*,*,*,*");
    }
}

#[test]
fn test_tokenize_empty() {
    let dict = build_test_dictionary(
        LEX_CSV.as_bytes(),
        MATRIX_DEF.as_bytes(),
        CHAR_DEF.as_bytes(),
        UNK_DEF.as_bytes(),
    );

    let tokenizer = Tokenizer::new(dict);
    let mut worker = tokenizer.new_worker();
    worker.reset_sentence("");
    worker.tokenize();
    assert_eq!(worker.num_tokens(), 0);
}

#[test]
fn test_tokenize_repeat() {
    let dict = build_test_dictionary(
        LEX_CSV.as_bytes(),
        MATRIX_DEF.as_bytes(),
        CHAR_DEF.as_bytes(),
        UNK_DEF.as_bytes(),
    );

    let tokenizer = Tokenizer::new(dict);
    let mut worker = tokenizer.new_worker();

    worker.reset_sentence("東京に行く");
    worker.tokenize();
    assert_eq!(worker.num_tokens(), 3);

    worker.reset_sentence("一橋大学大学院");
    worker.tokenize();
    assert_eq!(worker.num_tokens(), 1);

    worker.reset_sentence("");
    worker.tokenize();
    assert_eq!(worker.num_tokens(), 0);

    worker.reset_sentence("kampersanda");
    worker.tokenize();
    assert_eq!(worker.num_tokens(), 1);
}

#[test]
fn reset_and_mode_switch_invalidate_previous_results() {
    let tokenizer = Tokenizer::new(build_test_dictionary(
        LEX_CSV.as_bytes(),
        MATRIX_DEF.as_bytes(),
        CHAR_DEF.as_bytes(),
        UNK_DEF.as_bytes(),
    ));
    let mut worker = tokenizer.new_worker();
    worker.reset_sentence("東京都");
    worker.tokenize_nbest(2);
    assert!(worker.num_nbest_paths() > 0);
    worker.reset_sentence("京都");
    assert_eq!(worker.num_nbest_paths(), 0);
    assert!(worker.nbest_token_iter(0).is_none());
    worker.tokenize_nbest(2);
    worker.tokenize();
    assert_eq!(worker.num_nbest_paths(), 0);
    assert!(worker.nbest_token_iter(0).is_none());
    assert_eq!(worker.num_tokens(), 1);
    worker.tokenize();
    assert_eq!(worker.num_tokens(), 1);
    assert!(worker.lattice_snapshot().is_some());
    worker.tokenize_nbest(1);
    assert_eq!(worker.num_tokens(), 0);
    assert!(worker.lattice_snapshot().is_none());
    worker.reset_sentence("");
    worker.tokenize();
    assert_eq!(worker.num_nbest_paths(), 0);
    assert!(worker.lattice_snapshot().is_none());
}

#[test]
fn nbest_spans_and_lattice_deltas_with_spaces() {
    let dict = build_test_dictionary(
        b"a,0,0,2,a\nb,0,0,3,b\nab,0,0,9,ab\n",
        b"1 1\n0 0 1\n",
        b"DEFAULT 0 1 0\nSPACE 0 1 0\n0x0020 SPACE\n",
        b"DEFAULT,0,0,100,unknown\nSPACE,0,0,100,space\n",
    );
    let tokenizer = Tokenizer::new(dict).ignore_space(true).unwrap();
    let mut w = tokenizer.new_worker();
    w.reset_sentence("a b ");
    w.tokenize();
    let expected: Vec<_> = w
        .token_iter()
        .map(|t| (t.surface().to_owned(), t.range_byte(), t.range_char()))
        .collect();
    w.tokenize_nbest(1);
    let actual: Vec<_> = w
        .nbest_token_iter(0)
        .unwrap()
        .map(|t| (t.surface().to_owned(), t.range_byte(), t.range_char()))
        .collect();
    assert_eq!(expected, actual);
    assert_eq!(actual[0].0, "a");
    assert_eq!(actual[1].0, "b");
    w.reset_sentence("ab");
    w.tokenize();
    let lattice = w.lattice_snapshot().unwrap();
    assert_eq!(lattice.total_cost, 8);
    assert_eq!(
        lattice
            .nodes
            .iter()
            .find(|n| n.feature == "ab")
            .unwrap()
            .delta,
        3
    );
    assert!(
        lattice
            .best_path
            .iter()
            .all(|&i| lattice.nodes[i].delta == 0)
    );
    w.reset_sentence("京都");
    assert!(w.lattice_snapshot().is_none());
    assert_eq!(lattice.total_cost, 8);
}

#[cfg(feature = "legacy")]
#[test]
fn legacy_connectors_convert_without_reinterpreting_allocations() {
    let lex = b"a,1,1,2,a\nb,2,2,3,b\n";
    let chars = b"DEFAULT 0 1 0\nSPACE 0 1 0\n0x0020 SPACE\n";
    let unk = b"DEFAULT,0,0,100,unknown\nSPACE,0,0,100,space\n";
    let right = b"1\tAB,*,CD,*,EF,*,GH,*,IJ,*,KL,*,MN,*,OP,*,QR,*,ST\n2\tUV,*,WX,*,YZ,*,12,*,34,*,56,*,78,*,90,*,*,*,*\n";
    let left = b"1\tuv,*,wx,*,yz,*,12,*,34,*,56,*,78,*,90,*,*,*,*\n2\tab,*,cd,*,ef,*,gh,*,ij,*,kl,*,mn,*,op,*,qr,*,st\n";
    let cost = b"AB/ab\t-10\nCD/cd\t20\nEF/ef\t-30\nGH/gh\t40\nIJ/ij\t-50\nKL/kl\t60\nMN/mn\t-70\nOP/op\t80\nQR/qr\t-90\nST/st\t100\nUV/uv\t-110\n";
    for kind in 0..3 {
        let old = if kind == 0 {
            vibrato::SystemDictionaryBuilder::from_readers(
                lex.as_slice(),
                b"3 3\n0 0 1\n1 2 -7\n".as_slice(),
                chars.as_slice(),
                unk.as_slice(),
            )
            .unwrap()
        } else {
            vibrato::SystemDictionaryBuilder::from_readers_with_bigram_info(
                lex.as_slice(),
                right.as_slice(),
                left.as_slice(),
                cost.as_slice(),
                chars.as_slice(),
                unk.as_slice(),
                kind == 2,
            )
            .unwrap()
        };
        let mut bytes = Vec::new();
        old.write(&mut bytes).unwrap();
        // The bytes are generated by the trusted legacy builder immediately above.
        let converted = unsafe { Dictionary::from_legacy_reader(bytes.as_slice()) }.unwrap();
        let old_tokenizer = vibrato::Tokenizer::new(old).ignore_space(true).unwrap();
        let new_tokenizer = Tokenizer::new(converted).ignore_space(true).unwrap();
        let mut old_worker = old_tokenizer.new_worker();
        let mut new_worker = new_tokenizer.new_worker();
        for text in ["ab", "b a", "abba", "a x b"] {
            old_worker.reset_sentence(text);
            old_worker.tokenize();
            new_worker.reset_sentence(text);
            new_worker.tokenize();
            let old: Vec<_> = old_worker
                .token_iter()
                .map(|t| {
                    (
                        t.surface().to_owned(),
                        t.feature().to_owned(),
                        t.total_cost(),
                    )
                })
                .collect();
            let new: Vec<_> = new_worker
                .token_iter()
                .map(|t| {
                    (
                        t.surface().to_owned(),
                        t.feature().to_owned(),
                        t.total_cost(),
                    )
                })
                .collect();
            assert_eq!(old, new, "connector {kind}, input {text}");
        }
    }
}

#[test]
fn dictionary_preference_is_opt_in_for_tied_costs() {
    for (prefer, feature) in [(false, "unknown"), (true, "known")] {
        let dict = build_test_dictionary(
            b"a,0,0,0,known\n",
            b"1 1\n0 0 0\n",
            b"DEFAULT 1 1 0\n",
            b"DEFAULT,0,0,0,unknown\n",
        );
        let tokenizer = Tokenizer::new(dict).prefer_dictionary_on_tie(prefer);
        let mut w = tokenizer.new_worker();
        w.reset_sentence("a");
        w.tokenize();
        assert_eq!(w.token(0).feature(), feature);
        let snapshot = w.lattice_snapshot().unwrap();
        assert_eq!(snapshot.nodes.len(), 2);
        assert!(snapshot.nodes.iter().all(|n| n.delta == 0));
    }
}

#[test]
fn archived_system_accepts_independent_user_lexicon() {
    let inner = SystemDictionaryBuilder::from_readers(
        LEX_CSV.as_bytes(),
        MATRIX_DEF.as_bytes(),
        CHAR_DEF.as_bytes(),
        UNK_DEF.as_bytes(),
    )
    .unwrap();
    let mut bytes = Vec::new();
    inner.write(&mut bytes).unwrap();
    let base = Tokenizer::new(Dictionary::from_bytes(&bytes).unwrap());
    let updated = base.clone().with_user_lexicon(USER_CSV.as_bytes()).unwrap();
    let reference = Tokenizer::from_inner(
        inner
            .reset_user_lexicon_from_reader(Some(USER_CSV.as_bytes()))
            .unwrap(),
    );
    let mut original = base.new_worker();
    original.reset_sentence("京都東京都京都");
    original.tokenize();
    assert_eq!(original.num_tokens(), 3);
    let mut actual = updated.new_worker();
    let mut expected = reference.new_worker();
    for worker in [&mut actual, &mut expected] {
        worker.reset_sentence("京都東京都京都");
        worker.tokenize();
    }
    assert_eq!(actual.num_tokens(), 2);
    assert_eq!(
        actual
            .token_iter()
            .map(|t| (t.feature().to_owned(), t.word_cost()))
            .collect::<Vec<_>>(),
        expected
            .token_iter()
            .map(|t| (t.feature().to_owned(), t.word_cost()))
            .collect::<Vec<_>>()
    );
    let snapshot = actual.lattice_snapshot().unwrap();
    assert!(
        snapshot
            .nodes
            .iter()
            .any(|n| n.feature == actual.token(0).feature())
    );
    actual.tokenize_nbest(3);
    expected.tokenize_nbest(3);
    assert_eq!(actual.num_nbest_paths(), expected.num_nbest_paths());
    for path in 0..actual.num_nbest_paths() {
        assert_eq!(
            actual
                .nbest_token_iter(path)
                .unwrap()
                .map(|t| (t.feature().to_owned(), t.word_cost()))
                .collect::<Vec<_>>(),
            expected
                .nbest_token_iter(path)
                .unwrap()
                .map(|t| (t.feature().to_owned(), t.word_cost()))
                .collect::<Vec<_>>()
        );
    }
    assert!(
        base.with_user_lexicon("a,65535,65535,0,invalid\n".as_bytes())
            .is_err()
    );
}
