#!/bin/bash

CARGO_PROFILE_RELEASE_DEBUG=true cargo run --release -- -vv db ~/.local/share/testdata/testdata_small.csv | tee out.log
CARGO_PROFILE_RELEASE_DEBUG=true cargo flamegraph -- run '"20250522"' ~/.local/share/testdata/testdata_small.csv | tee -a out.log
