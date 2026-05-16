// Copyright (c) 2024, bzpack bridge for cpclib-crunchers
// BSD 2-Clause License.

#pragma once

#include "rust/cxx.h"
#include <cstdint>

/// Compress data using bzpack.
///
/// format_id: 0=LZM, 1=EF8, 2=BX0, 3=BX2
/// reverse: compress in reverse order (for backward decompression on Z80)
/// end_marker: add end-of-stream marker
/// extend_offset: extend offset range by 1
/// extend_length: extend block length by 1
/// natural_stream: produce natural stream without bit-inversion optimization
rust::Vec<uint8_t> bzpack_compress(
    rust::Slice<const uint8_t> data,
    uint8_t format_id,
    bool reverse,
    bool end_marker,
    bool extend_offset,
    bool extend_length,
    bool natural_stream);
