export FFMPEG_PKG_CONFIG_PATH=${PWD}/tmp/ffmpeg_build/lib/pkgconfig
export CARGO_INCREMENTAL=1
export RUSTFLAGS="-C target-feature=+sse3,+avx2,+fma -C target-cpu=native"
export RUST_LOG=info
