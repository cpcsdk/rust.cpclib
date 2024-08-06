cargo run -- -o 0x1000 tests/good_djnz.o 
cargo run -- -o 0x1000 -l START=0x1000 -l LOOP=0x1005 -- tests/good_djnz.o 