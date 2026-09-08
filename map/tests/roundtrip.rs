use std::{
    fs::File,
    io::Write,
    process::{Command, Stdio},
};
use vibrato_rkyv::{Dictionary, SystemDictionaryBuilder, Tokenizer};

#[test]
fn reorder_output_can_be_mapped_and_loaded() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("input.zst");
    let output = dir.path().join("output.zst");
    let mapping = dir.path().join("mapping");
    let dict = SystemDictionaryBuilder::from_readers(
        b"a,1,1,0,a\n".as_slice(),
        b"2 2\n0 0 0\n0 1 -2\n1 0 -3\n1 1 -4\n".as_slice(),
        b"DEFAULT 0 1 0\n".as_slice(),
        b"DEFAULT,0,0,100,unknown\n".as_slice(),
    )
    .unwrap();
    let mut writer = zstd::Encoder::new(File::create(&input).unwrap(), 1).unwrap();
    dict.write(&mut writer).unwrap();
    writer.finish().unwrap();
    let mut reorder = Command::new(env!("CARGO_BIN_EXE_reorder"))
        .arg("-i")
        .arg(&input)
        .arg("-o")
        .arg(&mapping)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    reorder.stdin.take().unwrap().write_all(b"aa\n").unwrap();
    let result = reorder.wait_with_output().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let result = Command::new(env!("CARGO_BIN_EXE_map"))
        .arg("-i")
        .arg(&input)
        .arg("-m")
        .arg(&mapping)
        .arg("-o")
        .arg(&output)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let tokenizer = Tokenizer::new(
        Dictionary::read(zstd::Decoder::new(File::open(output).unwrap()).unwrap()).unwrap(),
    );
    let mut worker = tokenizer.new_worker();
    worker.reset_sentence("aa");
    worker.tokenize();
    assert_eq!(
        worker.token_iter().map(|t| t.surface()).collect::<Vec<_>>(),
        vec!["a", "a"]
    );
    assert_eq!(worker.total_cost(), Some(-9));
}
