use std::error::Error;
use std::fs::File;
use std::io::{prelude::*, BufReader};
use std::path::PathBuf;

use rkyv::{access, deserialize, rancor::Error as RError};
use vibrato_rkyv::dictionary::{ArchivedDictionaryInner, DictionaryInner, MODEL_MAGIC};

use clap::Parser;

#[derive(Parser, Debug)]
#[clap(
    name = "map",
    about = "A program to edit connection ids with the reordered mapping."
)]
struct Args {
    /// System dictionary in binary to be edited (in zstd).
    #[clap(short = 'i', long)]
    sysdic_in: PathBuf,

    /// Basename of files of the reordered mappings.
    /// Two files *.lmap and *.rmap will be input.
    #[clap(short = 'm', long)]
    mapping_in: PathBuf,

    /// File to which the edited dictionary is output (in zstd).
    #[clap(short = 'o', long)]
    sysdic_out: PathBuf,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    eprintln!("Loading and deserializing the dictionary...");
    let mut reader = zstd::Decoder::new(File::open(args.sysdic_in)?)?;
    let mut header = [0u8; 32];
    reader.read_exact(&mut header)?;
    if &header[..MODEL_MAGIC.len()] != MODEL_MAGIC {
        return Err("The magic number of the input model mismatches.".into());
    }
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes)?;
    let mut dict_bytes = rkyv::util::AlignedVec::<16>::new();
    dict_bytes.extend_from_slice(&bytes);
    let archived = access::<ArchivedDictionaryInner, RError>(&dict_bytes)?;
    let mut dict_inner: DictionaryInner = deserialize::<_, RError>(archived)?;

    eprintln!("Loading and doing the mapping...");
    let lmap = {
        let mut filename = args.mapping_in.clone();
        filename.set_extension("lmap");
        load_mapping(File::open(filename)?)?
    };
    let rmap = {
        let mut filename = args.mapping_in.clone();
        filename.set_extension("rmap");
        load_mapping(File::open(filename)?)?
    };

    dict_inner = dict_inner.map_connection_ids_from_iter(lmap, rmap)?;

    eprintln!(
        "Writing the mapped system dictionary...: {:?}",
        args.sysdic_out
    );
    let mut f = zstd::Encoder::new(File::create(args.sysdic_out)?, 19)?;

    dict_inner.write(&mut f)?;
    f.finish()?;

    Ok(())
}

fn load_mapping<R>(rdr: R) -> Result<Vec<u16>, Box<dyn Error>>
where
    R: Read,
{
    let reader = BufReader::new(rdr);
    let lines = reader.lines();
    let mut ids = vec![];
    for line in lines {
        let line = line?;
        let cols: Vec<_> = line.split('\t').collect();
        ids.push(cols[0].parse()?);
    }
    Ok(ids)
}
