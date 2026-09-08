use std::{fs, path::Path, process::Command};
use vibrato_rkyv::{Dictionary, Tokenizer};

fn run(args: &[&str]) {
    let result = Command::new(env!("CARGO_BIN_EXE_compiler"))
        .args(args)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
fn full_build_and_dictgen_keep_user_lexicon() {
    let dir = tempfile::tempdir().unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../vibrato/src/tests/resources");
    let path = |name: &str| root.join(name).to_str().unwrap().to_owned();
    let output = dir.path().join("build");
    run(&[
        "full-build",
        "--seed-lexicon",
        &path("train_lex.csv"),
        "--seed-unk",
        &path("train_unk.def"),
        "--corpus",
        &path("corpus.txt"),
        "--char-def",
        &path("char.def"),
        "--feature-def",
        &path("feature.def"),
        "--rewrite-def",
        &path("rewrite.def"),
        "--user-lexicon-in",
        &path("user.csv"),
        "--max-iter",
        "2",
        "--out-dir",
        output.to_str().unwrap(),
    ]);
    let reader =
        zstd::Decoder::new(fs::File::open(output.join("system.dic.zst")).unwrap()).unwrap();
    let tokenizer = Tokenizer::new(Dictionary::read(reader).unwrap());
    let mut worker = tokenizer.new_worker();
    worker.reset_sentence("京都東京都");
    worker.tokenize();
    assert_eq!(worker.num_tokens(), 1);
    assert_eq!(worker.token(0).feature(), "カスタム名詞");
    let out = |name: &str| dir.path().join(name).to_str().unwrap().to_owned();
    run(&[
        "dictgen",
        "-i",
        output.join("model.bin.zst").to_str().unwrap(),
        "-l",
        &out("lex.csv"),
        "-m",
        &out("matrix.def"),
        "-u",
        &out("unk.def"),
        "--user-lexicon-in",
        &path("user.csv"),
        "--user-lexicon-out",
        &out("user.csv"),
    ]);
    let users = fs::read_to_string(out("user.csv")).unwrap();
    assert_eq!(users.lines().count(), 3);
    for word in ["京都東京都", "kampersanda", "ヴェネツィア"] {
        assert!(
            users
                .lines()
                .any(|line| line.starts_with(&format!("{word},")))
        );
    }
}
