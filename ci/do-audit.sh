#!/usr/bin/env bash

set -e

here="$(dirname "$0")"
src_root="$(readlink -f "${here}/..")"

cd "${src_root}"

source ci/rust-version.sh stable

cargo_audit_ignores=(
  # failure is officially deprecated/unmaintained
  #
  # Blocked on multiple upstream crates removing their `failure` dependency.
  --ignore RUSTSEC-2020-0036

  # `net2` crate has been deprecated; use `socket2` instead
  #
  # Blocked on https://github.com/paritytech/jsonrpc/issues/575
  --ignore RUSTSEC-2020-0016

  # stdweb is unmaintained
  #
  # Blocked on multiple upstream crates removing their `stdweb` dependency.
  --ignore RUSTSEC-2020-0056

  # Potential segfault in the time crate
  #
  # Blocked on multiple crates updating `time` to >= 0.2.23
  --ignore RUSTSEC-2020-0071

  # difference is unmaintained
  #
  # Blocked on predicates v1.0.6 removing its dependency on `difference`
  --ignore RUSTSEC-2020-0095

  # hyper is upgraded on master/v1.6 but not for v1.5
  --ignore RUSTSEC-2021-0020

  # generic-array: arr! macro erases lifetimes
  #
  # ed25519-dalek and libsecp256k1 not upgraded for v1.5
  --ignore RUSTSEC-2020-0146

)
scripts/cargo-for-all-lock-files.sh +"$rust_stable" audit "${cargo_audit_ignores[@]}"
