#!/bin/sh

pb-rs \
    --dont_use_cow \
    --output_directory src \
    "proto/authentication.proto" \
    "proto/keyexchange.proto" \
    "proto/mercury.proto" \
    "proto/metadata.proto"
rm src/mod.rs