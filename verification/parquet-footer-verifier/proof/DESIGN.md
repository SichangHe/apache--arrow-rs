# Verus harness design

- production correspondence
  - `FooterBytes` names the four little-endian length bytes and four terminal magic bytes consumed by `FooterTail::try_new`
  - `magic_kind` accepts exactly production `PAR1` and `PARE`
  - `decode_metadata_len` is the arithmetic expansion of production `u32::from_le_bytes`
  - `validate_footer` mirrors `FOOTER_SIZE + metadata_len`, the file-size comparison, and the metadata start calculation in `ParquetMetaDataReader::parse_metadata`
- verified result
  - accepted metadata range ends immediately before the eight-byte footer
  - range start plus metadata length equals range end
  - range end plus footer size equals file length
  - bad magic and oversized footer-plus-metadata are distinct errors
- executable witness
  - `main` runs accepted, bad-magic, and out-of-range examples
- admitted boundary
  - host `usize` is 64 bits, so production `u32` to `usize` conversion and addition match harness `u64` arithmetic
  - `PARE` is distinguished only; encryption is not modeled
  - metadata decoding and all other excluded subsystems remain outside the claim
