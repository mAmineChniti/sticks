# Demo

`demo/demo.sh` walks through the new-user flow: scaffold a CMake C++
project, build/run/test/clean through sticks, add and remove a pinned
dependency, and eject back to a plain project.

## Record an asciinema cast

`docs/demo.cast` is the committed recording. To re-record after
user-visible changes:

```bash
cargo build --release
asciinema rec --title "sticks demo" -c "STICKS=$PWD/target/release/sticks ./demo/demo.sh" docs/demo.cast
```

`docs/demo.gif` is the committed rendered version (generated with
[`agg`](https://github.com/asciinema/agg): `agg docs/demo.cast docs/demo.gif`)
and is embedded in the README. To embed a playable version on the web,
upload the cast to <https://asciinema.org/> and link the player URL instead —
or render an SVG locally with `svg-term-cli` (`svg-term --in docs/demo.cast
--out docs/demo.svg`) — note that asciinema 2.4 no longer ships a built-in
`svg` subcommand.
