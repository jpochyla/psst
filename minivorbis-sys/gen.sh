#!/bin/sh

bindgen \
    --whitelist-function "ov_.*" \
    --whitelist-var "OV_.*" \
    --whitelist-var "SEEK_.*" \
    --size_t-is-usize \
    bindings.h \
    -- -I minivorbis \
    > src/bindings.rs