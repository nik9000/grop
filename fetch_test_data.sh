#!/bin/bash

set -ex

rm -rf ~/.local/share/testdata
mkdir -p ~/.local/share/testdata/gsom
cd ~/.local/share/testdata/gsom
curl https://www.ncei.noaa.gov/data/global-summary-of-the-month/archive/gsom-latest.tar.gz | tar xz
cd ../
tail -q -n+2 gsom/* > testdata.csv
rm -rf gsom
head -n 10000000 testdata.csv > testdata_small.csv
