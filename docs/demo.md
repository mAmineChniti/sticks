# Demo

`demo/demo.sh` walks through the new-user flow: scaffold a CMake C++
project, build/run/test/clean through sticks, add and remove a pinned
dependency, and eject back to a plain project.

## Record an asciinema cast

```bash
asciinema rec -c ./demo/demo.sh docs/demo.cast
```

Render it for the README (SVG or GIF):

```bash
# SVG (embeds well, keeps text selectable)
asciinema svg docs/demo.cast > docs/demo.svg

# GIF
agg docs/demo.cast docs/demo.gif
```

Then embed in README.md, e.g.:

```markdown
[![demo](docs/demo.svg)](docs/demo.cast)
```

The cast file is committed so the demo stays reproducible; re-record after
user-visible changes.
