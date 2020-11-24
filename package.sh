#!/bin/bash
set -e

rm -r dist
mkdir dist
cargo build --release
cp target/release/dcs-hemmecs.exe dist
cp fonts/OFL.txt "dist/Font License.txt"
cp -r lua/Scripts dist
