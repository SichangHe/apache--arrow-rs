#!/bin/sh
set -eu

base='87cd2e526511ce75726bceb59033dfe4078a095d'
verus_sha256='562588748e3587043fa23361448b934ad88b32f7be3d6457938a7a417ec775f8'

if [ "$#" -ne 1 ]; then
    printf 'usage: %s /path/to/verus\n' "$0" >&2
    exit 2
fi

verus_bin=$1
script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repo_root=$(git -C "$script_dir" rev-parse --show-toplevel)
scratch=$(mktemp -d)
trap 'rm -r -- "$scratch"' EXIT

test "$(git -C "$repo_root" rev-parse HEAD^)" = "$base"
git -C "$repo_root" diff --quiet "$base" HEAD -- . \
    ':(exclude)verification/parquet-footer-verifier'
test "$(git -C "$repo_root" rev-parse "$base:parquet/src/file/metadata/footer_tail.rs")" = \
    'c33bc7a25c5a613687580143f8d852b65a607052'
test "$(git -C "$repo_root" rev-parse "$base:parquet/src/file/metadata/reader.rs")" = \
    '844ae747c7c98aca956a29400869138377611e10'
test "$(git -C "$repo_root" rev-parse "$base:parquet/src/file/serialized_reader.rs")" = \
    '113e5203c94bcb924a188d262cf0d12712a4c0ec'

(cd "$script_dir" && sha256sum -c reviewed-source.sha256)
test -x "$verus_bin"
test "$(sha256sum "$verus_bin" | awk '{print $1}')" = "$verus_sha256"

target_dir=${PARQUET_FOOTER_CARGO_TARGET_DIR:-$scratch/cargo-target}
export CARGO_TARGET_DIR="$target_dir"

cargo fmt --check --manifest-path "$script_dir/search/Cargo.toml"
cargo clippy --locked --release --all-targets \
    --manifest-path "$script_dir/search/Cargo.toml" -- -D warnings
cargo run --quiet --locked --release \
    --manifest-path "$script_dir/search/Cargo.toml" >"$scratch/search.txt"
grep -Fx 'status=no-counterexample' "$scratch/search.txt"
grep -Fx 'host_usize_bits=64' "$scratch/search.txt"
grep -Fx 'production_footer_cases=131084' "$scratch/search.txt"
grep -Fx 'magic_mutation_cases=2048' "$scratch/search.txt"
grep -Fx 'footer_slice_length_cases=17' "$scratch/search.txt"
grep -Fx 'ordinary_reader_range_cases=524336' "$scratch/search.txt"

cargo fmt --check --manifest-path "$script_dir/host/Cargo.toml"
cargo test --quiet --locked --manifest-path "$script_dir/host/Cargo.toml"
cargo clippy --locked --all-targets \
    --manifest-path "$script_dir/host/Cargo.toml" -- -D warnings

mkdir "$scratch/proof"
cp "$script_dir/proof/parquet_footer.rs" "$scratch/proof/"
(cd "$scratch/proof" && "$verus_bin" --no-cheating --time --output-json \
    --compile parquet_footer.rs >verus.json && ./parquet_footer)
grep -Eq '"success"[[:space:]]*:[[:space:]]*true' "$scratch/proof/verus.json"
grep -Eq '"verified"[[:space:]]*:[[:space:]]*5' "$scratch/proof/verus.json"
grep -Eq '"errors"[[:space:]]*:[[:space:]]*0' "$scratch/proof/verus.json"
if grep -Eq 'assume|admit|external_body|assume_specification' \
    "$script_dir/proof/parquet_footer.rs"; then
    exit 1
fi

printf '%s\n' 'checks=pass'
