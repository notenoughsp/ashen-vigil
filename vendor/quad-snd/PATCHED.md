# quad-snd 0.2.8, patched for Ashen Vigil

A copy of [quad-snd](https://github.com/not-fl3/quad-snd) 0.2.8 (MIT / Apache-2.0), used through
`[patch.crates-io]` in the game's Cargo.toml. The only change is in `src/mixer.rs`:
`load_samples_from_file` copies data that starts with `RAW_F32_MAGIC` (44100 Hz stereo f32)
instead of parsing it, so loading a long recorded song no longer stalls a frame.
