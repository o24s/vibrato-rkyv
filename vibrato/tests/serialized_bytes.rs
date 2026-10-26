#![cfg(not(target_family = "wasm"))]

use std::fs;
use std::io::{self, Write};
use std::sync::{Arc, Barrier, OnceLock};
use vibrato_rkyv::dictionary::{DictionaryInner, LoadMode, MODEL_MAGIC};
use vibrato_rkyv::{Dictionary, SystemDictionaryBuilder};

fn inner(payload: &[u8]) -> DictionaryInner {
    let mut csv = String::from("dog,0,0,1,dog\n");
    if payload.is_empty() {
        csv.push_str("cat,0,0,1,cat\n");
    }
    for (index, chunk) in payload.chunks(256).enumerate() {
        let feature = chunk.iter().map(|b| format!("{b:02x}")).collect::<String>();
        csv.push_str(&format!("cat{index},0,0,1,cat{feature}\n"));
    }
    SystemDictionaryBuilder::from_readers(
        csv.as_bytes(),
        b"1 1\n0 0 0\n".as_slice(),
        b"DEFAULT 0 1 0\n".as_slice(),
        b"DEFAULT,0,0,100,unknown\n".as_slice(),
    )
    .unwrap()
}

fn bytes(payload: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::new();
    inner(payload).write(&mut bytes).unwrap();
    bytes
}

fn copy(dictionary: &Dictionary) -> Vec<u8> {
    let mut copied = Vec::new();
    assert_eq!(
        dictionary.copy_serialized_bytes(&mut copied).unwrap(),
        Some(copied.len() as u64)
    );
    copied
}

#[hegel::test(test_cases = 100, derandomize = true)]
fn file_copies_preserve_exact_serialization_and_padding(tc: hegel::TestCase) {
    use hegel::generators as g;
    let payload = tc.draw(g::vecs(g::integers::<u8>()));
    let mut original = bytes(&payload);
    let start = MODEL_MAGIC.len();
    let end = start.next_multiple_of(16);
    for byte in &mut original[start..end] {
        *byte = tc.draw(g::integers::<u8>());
    }
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("dictionary.dic");
    fs::write(&path, &original).unwrap();
    for mode in [LoadMode::Validate, LoadMode::TrustCache] {
        let dictionary = Dictionary::from_path(&path, mode).unwrap();
        assert_eq!(copy(&dictionary), original);
        assert_eq!(copy(&dictionary), original);
    }
    let dictionary = unsafe { Dictionary::from_path_unchecked(&path) }.unwrap();
    assert_eq!(copy(&dictionary), original);
}

struct PartialWriter {
    bytes: Vec<u8>,
    chunk: usize,
    interrupt: bool,
}
impl Write for PartialWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.interrupt = !self.interrupt;
        if self.interrupt {
            return Err(io::ErrorKind::Interrupted.into());
        }
        std::thread::yield_now();
        let n = self.chunk.min(bytes.len());
        self.bytes.extend_from_slice(&bytes[..n]);
        Ok(n)
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[hegel::test(test_cases = 100, derandomize = true)]
fn interrupted_and_partial_writes_preserve_all_bytes(tc: hegel::TestCase) {
    use hegel::generators as g;
    let original = bytes(&tc.draw(g::vecs(g::integers::<u8>())));
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("dictionary.dic");
    fs::write(&path, &original).unwrap();
    let dictionary = Dictionary::from_path(path, LoadMode::Validate).unwrap();
    let mut writer = PartialWriter {
        bytes: Vec::new(),
        chunk: usize::from(tc.draw(g::integers::<u8>())) + 1,
        interrupt: false,
    };
    assert_eq!(
        dictionary.copy_serialized_bytes(&mut writer).unwrap(),
        Some(original.len() as u64)
    );
    assert_eq!(writer.bytes, original);
}

struct FailingWriter {
    bytes: Vec<u8>,
    limit: usize,
}
impl Write for FailingWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.bytes.len() == self.limit {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        let n = bytes.len().min(self.limit - self.bytes.len());
        self.bytes.extend_from_slice(&bytes[..n]);
        Ok(n)
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[hegel::test(test_cases = 100, derandomize = true)]
fn failed_destinations_do_not_change_subsequent_copy_positions(tc: hegel::TestCase) {
    use hegel::generators as g;
    let original = bytes(&tc.draw(g::vecs(g::integers::<u8>())));
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("dictionary.dic");
    fs::write(&path, &original).unwrap();
    let dictionary = Dictionary::from_path(path, LoadMode::Validate).unwrap();
    let limit = tc.draw(g::integers::<u32>()) as usize % original.len();
    let mut writer = FailingWriter {
        bytes: Vec::new(),
        limit,
    };
    assert_eq!(
        dictionary
            .copy_serialized_bytes(&mut writer)
            .unwrap_err()
            .kind(),
        io::ErrorKind::PermissionDenied
    );
    assert_eq!(writer.bytes, original[..limit]);
    assert_eq!(copy(&dictionary), original);
}

#[hegel::test(test_cases = 40, derandomize = true)]
fn concurrent_copies_have_independent_positions(tc: hegel::TestCase) {
    use hegel::generators as g;
    let original = bytes(&tc.draw(g::vecs(g::integers::<u8>())));
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("dictionary.dic");
    fs::write(&path, &original).unwrap();
    let dictionary = Arc::new(Dictionary::from_path(path, LoadMode::Validate).unwrap());
    let workers = usize::from(tc.draw(g::integers::<u8>())) % 8 + 1;
    let barrier = Arc::new(Barrier::new(workers));
    std::thread::scope(|scope| {
        let handles = (0..workers)
            .map(|_| {
                let dictionary = &dictionary;
                let barrier = &barrier;
                scope.spawn(move || {
                    barrier.wait();
                    let mut writer = PartialWriter {
                        bytes: Vec::new(),
                        chunk: 37,
                        interrupt: false,
                    };
                    dictionary.copy_serialized_bytes(&mut writer).unwrap();
                    writer.bytes
                })
            })
            .collect::<Vec<_>>();
        for handle in handles {
            assert_eq!(handle.join().unwrap(), original);
        }
    });
}

#[test]
fn non_file_variants_do_not_write() {
    static BYTES: OnceLock<Vec<u8>> = OnceLock::new();
    let original = BYTES.get_or_init(|| bytes(&[]));
    static UNALIGNED: OnceLock<Vec<u8>> = OnceLock::new();
    let unaligned = UNALIGNED.get_or_init(|| {
        let mut bytes = vec![0];
        bytes.extend_from_slice(original);
        bytes
    });
    let dictionaries = [
        Dictionary::from_bytes(&unaligned[1..]).unwrap(),
        unsafe { Dictionary::from_bytes_unchecked(&unaligned[1..]) }.unwrap(),
        Dictionary::from_inner(inner(&[])),
        Dictionary::read(original.as_slice()).unwrap(),
        Dictionary::from_bytes(original).unwrap(),
        Dictionary::from_static_bytes(original).unwrap(),
        unsafe { Dictionary::from_bytes_unchecked(original) }.unwrap(),
    ];
    struct NoWrites;
    impl Write for NoWrites {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            panic!("no backing file")
        }
        fn flush(&mut self) -> io::Result<()> {
            panic!("no backing file")
        }
    }
    for dictionary in dictionaries {
        assert_eq!(
            dictionary.copy_serialized_bytes(&mut NoWrites).unwrap(),
            None
        );
    }
}

#[test]
fn zero_progress_destination_reports_write_zero() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("dictionary.dic");
    fs::write(&path, bytes(&[])).unwrap();
    let dictionary = Dictionary::from_path(path, LoadMode::Validate).unwrap();
    struct Zero;
    impl Write for Zero {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Ok(0)
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    assert_eq!(
        dictionary
            .copy_serialized_bytes(&mut Zero)
            .unwrap_err()
            .kind(),
        io::ErrorKind::WriteZero
    );
}

#[cfg(unix)]
#[test]
fn replacement_and_unlink_preserve_the_opened_file() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("dictionary.dic");
    let original = bytes(b"original");
    fs::write(&path, &original).unwrap();
    let dictionary = Dictionary::from_path(&path, LoadMode::Validate).unwrap();
    let old = root.path().join("old.dic");
    fs::rename(&path, &old).unwrap();
    fs::write(&path, bytes(b"replacement")).unwrap();
    fs::remove_file(old).unwrap();
    assert_eq!(copy(&dictionary), original);
}

#[test]
fn appended_bytes_are_outside_the_loaded_serialization() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("dictionary.dic");
    let original = bytes(&[]);
    fs::write(&path, &original).unwrap();
    let dictionary = Dictionary::from_path(&path, LoadMode::Validate).unwrap();
    fs::OpenOptions::new()
        .append(true)
        .open(path)
        .unwrap()
        .write_all(b"not mapped")
        .unwrap();
    assert_eq!(copy(&dictionary), original);
}

#[test]
fn truncating_the_backing_file_reports_unexpected_eof() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("dictionary.dic");
    fs::write(&path, bytes(&[])).unwrap();
    let dictionary = Dictionary::from_path(&path, LoadMode::Validate).unwrap();
    fs::OpenOptions::new()
        .write(true)
        .open(path)
        .unwrap()
        .set_len(16)
        .unwrap();
    assert_eq!(
        dictionary
            .copy_serialized_bytes(&mut Vec::new())
            .unwrap_err()
            .kind(),
        io::ErrorKind::UnexpectedEof
    );
}

#[test]
fn concurrent_copies_span_multiple_copy_buffers() {
    let feature = "x".repeat(2048);
    let csv = (0..1200)
        .map(|i| format!("cat,0,0,1,{feature}{i}\n"))
        .collect::<String>();
    let inner = SystemDictionaryBuilder::from_readers(
        csv.as_bytes(),
        b"1 1\n0 0 0\n".as_slice(),
        b"DEFAULT 0 1 0\n".as_slice(),
        b"DEFAULT,0,0,100,unknown\n".as_slice(),
    )
    .unwrap();
    let mut original = Vec::new();
    inner.write(&mut original).unwrap();
    assert!(original.len() > 2 << 20);
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("dictionary.dic");
    fs::write(&path, &original).unwrap();
    let dictionary = Dictionary::from_path(path, LoadMode::Validate).unwrap();
    std::thread::scope(|scope| {
        let handles = (0..4)
            .map(|_| scope.spawn(|| copy(&dictionary)))
            .collect::<Vec<_>>();
        for handle in handles {
            assert!(handle.join().unwrap() == original);
        }
    });
}

#[test]
fn native_zstd_copies_match_the_decompressed_file_on_cold_and_warm_loads() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("dictionary.dic.zst");
    let original = bytes(&[]);
    fs::write(
        &path,
        zstd::stream::encode_all(original.as_slice(), 1).unwrap(),
    )
    .unwrap();
    for _ in 0..2 {
        let dictionary = Dictionary::from_zstd_with_options(
            &path,
            root.path().join("cache"),
            #[cfg(feature = "legacy")]
            true,
        )
        .unwrap();
        assert_eq!(copy(&dictionary), original);
    }
}

#[cfg(feature = "legacy")]
#[test]
fn legacy_sources_and_converted_caches_have_distinct_serializations() {
    let inner = vibrato::SystemDictionaryBuilder::from_readers(
        b"cat,0,0,1,cat\n".as_slice(),
        b"1 1\n0 0 0\n".as_slice(),
        b"DEFAULT 0 1 0\n".as_slice(),
        b"DEFAULT,0,0,100,unknown\n".as_slice(),
    )
    .unwrap();
    let mut original = Vec::new();
    inner.write(&mut original).unwrap();
    let root = tempfile::tempdir().unwrap();
    let plain = root.path().join("legacy.dic");
    fs::write(&plain, &original).unwrap();
    for dictionary in [
        Dictionary::from_path(&plain, LoadMode::Validate).unwrap(),
        unsafe { Dictionary::from_path_unchecked(&plain) }.unwrap(),
        unsafe { Dictionary::from_legacy_reader(original.as_slice()) }.unwrap(),
    ] {
        assert_eq!(
            dictionary.copy_serialized_bytes(&mut Vec::new()).unwrap(),
            None
        );
    }
    let path = root.path().join("legacy.dic.zst");
    fs::write(
        &path,
        zstd::stream::encode_all(original.as_slice(), 1).unwrap(),
    )
    .unwrap();
    let cold = Dictionary::from_zstd_with_options(&path, root.path().join("cache"), true).unwrap();
    assert_eq!(cold.copy_serialized_bytes(&mut Vec::new()).unwrap(), None);
    let warm = Dictionary::from_zstd_with_options(&path, root.path().join("cache"), true).unwrap();
    let converted = copy(&warm);
    assert!(converted.starts_with(MODEL_MAGIC));
    assert_ne!(converted, original);
}
