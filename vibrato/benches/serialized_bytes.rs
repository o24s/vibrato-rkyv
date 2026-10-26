use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use sha2::{Digest, Sha256};
use std::io::{self, Write};
use vibrato_rkyv::dictionary::LoadMode;
use vibrato_rkyv::{Dictionary, SystemDictionaryBuilder};

struct HashingSink(Sha256);

impl Write for HashingSink {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.update(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn bench_serialized_bytes(c: &mut Criterion) {
    let feature = "x".repeat(2048);
    let lexicon = (0..2048)
        .map(|index| format!("cat,0,0,1,{feature}{index}\n"))
        .collect::<String>();
    let inner = SystemDictionaryBuilder::from_readers(
        lexicon.as_bytes(),
        b"1 1\n0 0 0\n".as_slice(),
        b"DEFAULT 0 1 0\n".as_slice(),
        b"DEFAULT,0,0,100,unknown\n".as_slice(),
    )
    .unwrap();
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("dictionary.dic");
    let mut file = std::fs::File::create(&path).unwrap();
    inner.write(&mut file).unwrap();
    let length = file.metadata().unwrap().len();
    let dictionary = Dictionary::from_path(path, LoadMode::Validate).unwrap();
    let mut group = c.benchmark_group("SerializedBytes");
    group.throughput(Throughput::Bytes(length));
    group.bench_function("file-backed-sha256", |b| {
        b.iter(|| {
            let mut sink = HashingSink(Sha256::new());
            assert_eq!(
                dictionary.copy_serialized_bytes(&mut sink).unwrap(),
                Some(length)
            );
            std::hint::black_box(sink.0.finalize())
        })
    });
    group.finish();
}

criterion_group!(benches, bench_serialized_bytes);
criterion_main!(benches);
