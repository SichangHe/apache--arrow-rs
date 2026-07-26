# Bounded Parquet footer verifier experiment

This portable experiment is fixed to Apache Arrow Rust base
`87cd2e526511ce75726bceb59033dfe4078a095d`. It changes no production source and
does not claim an upstream defect or full-reader correctness.

## Boundary

The experiment covers only the ordinary Parquet metadata-read path's terminal
`PAR1`/`PARE` magic check, unsigned little-endian metadata length, and checked
metadata interval. It excludes encodings, compression, page decoding, schemas,
metadata decoding, I/O correctness, encryption semantics, 32-bit arithmetic,
distributed execution, and end-to-end claims.

The production link at the fixed base is:

- `parquet/src/file/metadata/footer_tail.rs`, blob
  `c33bc7a25c5a613687580143f8d852b65a607052`
- `parquet/src/file/metadata/reader.rs`, blob
  `844ae747c7c98aca956a29400869138377611e10`
- `parquet/src/file/serialized_reader.rs`, blob
  `113e5203c94bcb924a188d262cf0d12712a4c0ec`

`FooterTail::try_new` checks the four magic bytes and decodes the first four
bytes with `u32::from_le_bytes`. `ParquetMetaDataReader::parse_metadata`
compares `8 + metadata_len` with the file size before deriving the metadata
start. The Verus harness mirrors only those operations; the two Rust helper
crates call the pinned production APIs directly.

## Preserved evidence

The proof, Rust harnesses, design note, and lockfiles are byte-for-byte copies
of the reviewed local experiment. `reviewed-source.sha256` records their
identities. The helper manifests retain the reviewed package names,
dependencies, feature selection, and edition. Their only portability changes
are an empty nested workspace and replacing the original private absolute
`parquet` path with `../../../parquet`.

The bounded x86-64 search recorded no counterexample in 657,485
production-linked cases: 131,084 footer cases, 2,048 one-byte magic mutations,
17 footer slice lengths, and 524,336 ordinary-reader range cases. The focused
host suite recorded five passing tests. Verus
`0.2025.03.28.892067f` recorded `5 verified, 0 errors` with `--no-cheating`, and
the compiled proof executable exited successfully. These results apply only to
the boundary above.

## Reproduce

Use an x86-64 Linux checkout of this exact commit and the repository Rust
toolchain. Obtain the Verus release asset
`verus-0.2025.03.28.892067f-x86-linux.zip` from the
[upstream release](https://github.com/verus-lang/verus/releases/tag/release/0.2025.03.28.892067f).
The archive SHA-256 is
`f4e6958d7bf9e6a2d82d0a8ecf57474722694228885e6d37eb1d4cd86983c7e5`; the
extracted `verus` binary SHA-256 is
`562588748e3587043fa23361448b934ad88b32f7be3d6457938a7a417ec775f8`.

Run:

```console
./verification/parquet-footer-verifier/check.sh /path/to/verus
```

The check validates the base and production blobs, checks the preserved source
identities, reruns formatting and warnings-denied Clippy, repeats the bounded
search and five host tests, verifies the proof with `--no-cheating`, and runs
the compiled proof witness. Build and verifier output is confined to a
temporary directory.
