# asciipet

A tiny 3D character that hops, blinks, waves and dances in your terminal.

It is not a GIF converted to ASCII. Every frame is ray marched from a signed
distance field (SDF) model, lit, and printed as characters with ANSI colour.

```
                            B#$$##B0                  &#$$#&B
                           8##$$##&BQ                &#$$$##B8
                           0B&##$$##&BB&&&&&&&&BBBB88B##$#&B8Q:
                            BB&&##################&&&&&BBB880n
                         B&&#####$$$$$$$$$$$$$$#######&&&&BBB880Q
                      BB&&####$$$$$$$$$$$$$$$$$$$######&&&&BBB8800Qn
                     BB&&&#####$$$$    $$$$$$$$$######   &&BBB88800Qn
           #$##&&&BB#&B&&&######$$$    $$$$$$$$######    &&BBB88800Qn8BB&##$$#B
          B#########$#B&&&&########$  $$$$$$#########&   &BBBB8800QQQ&$$$$$$$#BQ
              :nQQQ08BBBBB&&&&&##################&&&&&&BBBB888800QQnnQ0QQQnn
                     Q0888BBBB&&&&&&&&&&&&&&&&&&&&BBBBBB888800QQQnnn:
                        :nQQ000088888888888888888800000QQQQQQQnnn:
                               Q#$$#B0QQQQQQQQQQQQQnQQ8&&B0
                            0B##$$##&B8Q:   ::     8&#####&B8Q:

                          .......::::::::::::::::::::::::.......
```

## Install

From source (needs [Rust](https://rustup.rs)):

```bash
cargo install --git https://github.com/2miwon/ascii-obj-render
```

Prebuilt binaries for macOS, Linux and Windows are attached to each
[GitHub release](https://github.com/2miwon/ascii-obj-render/releases).

## Usage

```bash
asciipet                    # cycle through every animation, q / Esc to quit
asciipet --anim dance       # idle | hop | wave | dance | auto
asciipet --duration 3       # play for 3 seconds then exit (nice in .zshrc)
asciipet --fps 60 --speed 1.5
asciipet --mono             # plain characters, no colour
asciipet --once --time 0.3 --width 100 --height 40   # print a single frame
```

Colour is auto-detected: truecolor when `COLORTERM` says so, the 256-colour
palette otherwise (e.g. macOS Terminal.app), none when `NO_COLOR` is set.
Override with `--color truecolor|256|none`.

## How it works

1. **Model** (`src/character.rs`): the body, ears, arms and feet are ellipsoids
   and capsules joined with a smooth minimum, so they blend like clay.
2. **Animation**: each frame computes a `Pose` from time `t`: hop height,
   squash and stretch, body lean and turn, ear and arm angles, blink.
3. **Ray marching** (`src/render.rs`): every terminal cell shoots a ray and
   steps forward by the SDF distance until it touches the surface.
4. **Lighting**: the surface normal comes from the SDF gradient; diffuse plus a
   facing term gives a brightness in `0..1`.
5. **Glyphs**: brightness picks a character from `.:nQ08B&#$@`. Face features
   (eyes, mouth, cheeks, inner ears) are decals painted in the body's local
   space, so they turn and squash with it. Eyes become `-` when blinking.

Cells are about twice as tall as they are wide, so horizontal ray directions are
scaled by 0.5. Rows are rendered in parallel with rayon; a 220×70 frame takes a
few milliseconds.

## Roadmap

- Characters defined in TOML files, so anyone can add one without writing Rust
- `--react success|fail` for shell hooks
- `--record out.gif` for exporting animations
- Homebrew tap

## Contributing characters

Original designs only. Characters owned by someone else (anime, games,
mascots) will not be merged, even as fan art, because they would put the whole
repository at risk of a takedown.

## License

MIT
