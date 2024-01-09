#pragma once

#include "rust/cxx.h"


rust::Vec<unsigned char>  compress_for_basm(rust::Slice<const unsigned char> data, int iterations, bool log);
