use bytes::Bytes;
use parquet::errors::{ParquetError, Result as ParquetResult};
use parquet::file::metadata::{FooterTail, ParquetMetaDataReader};
use parquet::file::reader::{ChunkReader, Length};
use std::io::Cursor;

const FOOTER_SIZE: u64 = 8;
const PLAIN_MAGIC: [u8; 4] = *b"PAR1";
const ENCRYPTED_MAGIC: [u8; 4] = *b"PARE";

struct ProbeReader {
    file_len: u64,
    footer: [u8; FOOTER_SIZE as usize],
}

impl Length for ProbeReader {
    fn len(&self) -> u64 {
        self.file_len
    }
}

impl ChunkReader for ProbeReader {
    type T = Cursor<Vec<u8>>;

    fn get_read(&self, start: u64) -> ParquetResult<Self::T> {
        let expected = self
            .file_len
            .checked_sub(FOOTER_SIZE)
            .ok_or_else(|| ParquetError::General("unexpected footer read".to_owned()))?;
        if start != expected {
            return Err(ParquetError::General(format!(
                "footer-read:{start}:{expected}"
            )));
        }
        Ok(Cursor::new(self.footer.to_vec()))
    }

    fn get_bytes(&self, start: u64, length: usize) -> ParquetResult<Bytes> {
        Err(ParquetError::General(format!(
            "probe-range:{start}:{length}"
        )))
    }
}

fn footer(metadata_len: u32, magic: [u8; 4]) -> [u8; FOOTER_SIZE as usize] {
    let mut result = [0; FOOTER_SIZE as usize];
    result[..4].copy_from_slice(&metadata_len.to_le_bytes());
    result[4..].copy_from_slice(&magic);
    result
}

fn check_tail(metadata_len: u32, magic: [u8; 4], encrypted: bool) -> Result<(), String> {
    let actual = FooterTail::try_from(footer(metadata_len, magic))
        .map_err(|error| format!("valid footer rejected for length {metadata_len}: {error}"))?;
    if actual.metadata_length() != metadata_len as usize {
        return Err(format!(
            "length mismatch: input={metadata_len} output={}",
            actual.metadata_length()
        ));
    }
    if actual.is_encrypted_footer() != encrypted {
        return Err(format!(
            "encryption flag mismatch for length {metadata_len}"
        ));
    }
    Ok(())
}

fn check_magic_mutations() -> Result<u64, String> {
    let mut cases = 0;
    for accepted in [PLAIN_MAGIC, ENCRYPTED_MAGIC] {
        for index in 0..accepted.len() {
            for byte in u8::MIN..=u8::MAX {
                let mut candidate = accepted;
                candidate[index] = byte;
                let actual = FooterTail::try_from(footer(0x7856_3412, candidate)).is_ok();
                let expected = candidate == PLAIN_MAGIC || candidate == ENCRYPTED_MAGIC;
                if actual != expected {
                    return Err(format!("magic mismatch for {candidate:?}"));
                }
                cases += 1;
            }
        }
    }
    Ok(cases)
}

fn check_reader_case(metadata_len: u32, file_len: u64) -> Result<(), String> {
    let total = FOOTER_SIZE + u64::from(metadata_len);
    let source = ProbeReader {
        file_len,
        footer: footer(metadata_len, PLAIN_MAGIC),
    };
    let mut reader = ParquetMetaDataReader::new();
    let error = reader
        .try_parse(&source)
        .expect_err("probe always stops before metadata decoding");

    if file_len < FOOTER_SIZE {
        if !matches!(error, ParquetError::EOF(_)) || reader.metadata_size().is_some() {
            return Err(format!(
                "short-file mismatch: metadata={metadata_len} file={file_len} error={error} size={:?}",
                reader.metadata_size()
            ));
        }
        return Ok(());
    }

    if reader.metadata_size() != Some(total as usize) {
        return Err(format!(
            "metadata-size mismatch: metadata={metadata_len} file={file_len} size={:?}",
            reader.metadata_size()
        ));
    }

    if total > file_len {
        if !matches!(error, ParquetError::EOF(_)) {
            return Err(format!(
                "oversized metadata was not rejected: metadata={metadata_len} file={file_len} error={error}"
            ));
        }
        return Ok(());
    }

    let start = file_len - total;
    let expected = format!("Parquet error: probe-range:{start}:{metadata_len}");
    if error.to_string() != expected {
        return Err(format!(
            "range mismatch: metadata={metadata_len} file={file_len} expected={expected} actual={error}"
        ));
    }
    if start + u64::from(metadata_len) != file_len - FOOTER_SIZE {
        return Err(format!(
            "range invariant mismatch: metadata={metadata_len} file={file_len} start={start}"
        ));
    }
    Ok(())
}

fn main() -> Result<(), String> {
    if usize::BITS != 64 {
        return Err(format!("unsupported host pointer width: {}", usize::BITS));
    }

    let high_lengths = [
        u32::from(u16::MAX) + 1,
        u32::MAX - 9,
        u32::MAX - 8,
        u32::MAX - 7,
        u32::MAX - 1,
        u32::MAX,
    ];

    let mut footer_cases = 0_u64;
    for metadata_len in u32::from(u16::MIN)..=u32::from(u16::MAX) {
        check_tail(metadata_len, PLAIN_MAGIC, false)?;
        check_tail(metadata_len, ENCRYPTED_MAGIC, true)?;
        footer_cases += 2;
    }
    for metadata_len in high_lengths {
        check_tail(metadata_len, PLAIN_MAGIC, false)?;
        check_tail(metadata_len, ENCRYPTED_MAGIC, true)?;
        footer_cases += 2;
    }

    let magic_cases = check_magic_mutations()?;
    let mut slice_length_cases = 0_u64;
    for length in 0..=16 {
        let mut bytes = footer(28, PLAIN_MAGIC).to_vec();
        bytes.resize(length, 0);
        let actual = FooterTail::try_from(bytes.as_slice()).is_ok();
        if actual != (length == FOOTER_SIZE as usize) {
            return Err(format!("footer slice length mismatch: {length}"));
        }
        slice_length_cases += 1;
    }

    let mut range_cases = 0_u64;
    for metadata_len in u32::from(u16::MIN)..=u32::from(u16::MAX) {
        let total = FOOTER_SIZE + u64::from(metadata_len);
        for file_len in [
            0,
            7,
            8,
            total.saturating_sub(1),
            total,
            total.saturating_add(1),
            total / 2,
            u64::MAX,
        ] {
            check_reader_case(metadata_len, file_len)?;
            range_cases += 1;
        }
    }
    for metadata_len in high_lengths {
        let total = FOOTER_SIZE + u64::from(metadata_len);
        for file_len in [
            0,
            7,
            8,
            total.saturating_sub(1),
            total,
            total.saturating_add(1),
            total / 2,
            u64::MAX,
        ] {
            check_reader_case(metadata_len, file_len)?;
            range_cases += 1;
        }
    }

    println!("status=no-counterexample");
    println!("host_usize_bits={}", usize::BITS);
    println!("production_footer_cases={footer_cases}");
    println!("magic_mutation_cases={magic_cases}");
    println!("footer_slice_length_cases={slice_length_cases}");
    println!("ordinary_reader_range_cases={range_cases}");
    Ok(())
}
