#! /usr/bin/env bash

set -euo pipefail

cd "$(dirname ${BASH_SOURCE[0]})"
cd ../

cargo test --release
cargo build --release

# TODO(shelbyd): Deduplicate bin location?
cp target/release/gentle "$HOME/.gentle/gentle"
