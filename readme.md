# waybound

hot boundaries (corners and edges) for wayland. a simple util that triggers
commands when the pointer enters configured corners and edges

![waybound demo](https://github.com/user-attachments/assets/8c5c5399-60ce-4014-a9a7-72f70da13e52)

## usage

```sh
waybound --rule top=alacritty --rule top-right="notify-send hello"
```

options:

- `-c, --config <config>`: load config from this file
- `-r, --rule <boundary=command>`: add a boundary rule, like top-left=... or right=...
- `-s, --size <pixels>`: boundary size in pixels [default: 5]
- `--debug`: show debug output and make boundary overlays visible

## config

by default, waybound looks for `${XDG_CONFIG_HOME:-$HOME/.config}/waybound/waybound.toml`

sample config:

```toml
debug = true
size = 15

[[boundaries]]
boundary = "top"
command = "alacritty"

[[boundaries]]
boundary = "top-right"
command = "notify-send $HOME"
```

cli flags override config rules for the same boundary. default boundary size
is 5 px. supported boundaries include `top-left`, `top-right`, `bottom-left`,
`bottom-right`, `top`, `bottom`, `left`, and `right`

## license

MIT
