cargo build --release
cd tests/asm/roudoudou
perf record --call-graph=dwarf ../../../../target/release/basm  rasm_sprites.asm
perf record ../../../../target/release/basm  rasm_sprites.asm
perf script > /tmp/test.perf

flamegraph ../../../../target/release/basm  rasm_sprites.asm
#perf report --hierarchy -M intel