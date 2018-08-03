#!/bin/bash
mkdir export
cp target/release/taiko-copy export/taiko-copy
rsync -avp assets/* export/assets
zip -r taiko-copy-export.zip export/*
