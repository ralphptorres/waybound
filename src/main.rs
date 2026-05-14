use clap::Parser;
use wayland_client::{Connection, EventQueue};
use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1;

mod wayland;
use crate::wayland::WaylandState;

#[derive(Clone, Debug)]
struct Placement {
    name: String,
    anchor: zwlr_layer_surface_v1::Anchor,
    width: u32,
    height: u32,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short = 'c', long)]
    command: String,

    #[arg(short = 'n', long, default_value = "top-left")]
    corner: String,

    #[arg(long)]
    debug: bool,
}

fn parse_placement(corner: &str) -> Placement {
    use zwlr_layer_surface_v1::Anchor;
    let lower = corner.to_lowercase();
    match lower.as_str() {
        "top-left" => Placement {
            name: lower,
            anchor: Anchor::Top | Anchor::Left,
            width: 10,
            height: 10,
        },
        "top-right" => Placement {
            name: lower,
            anchor: Anchor::Top | Anchor::Right,
            width: 10,
            height: 10,
        },
        "bottom-left" => Placement {
            name: lower,
            anchor: Anchor::Bottom | Anchor::Left,
            width: 10,
            height: 10,
        },
        "bottom-right" => Placement {
            name: lower,
            anchor: Anchor::Bottom | Anchor::Right,
            width: 10,
            height: 10,
        },
        "top" => Placement {
            name: lower,
            anchor: Anchor::Top | Anchor::Left | Anchor::Right,
            width: 0,
            height: 10,
        },
        "bottom" => Placement {
            name: lower,
            anchor: Anchor::Bottom | Anchor::Left | Anchor::Right,
            width: 0,
            height: 10,
        },
        "left" => Placement {
            name: lower,
            anchor: Anchor::Left | Anchor::Top | Anchor::Bottom,
            width: 10,
            height: 0,
        },
        "right" => Placement {
            name: lower,
            anchor: Anchor::Right | Anchor::Top | Anchor::Bottom,
            width: 10,
            height: 0,
        },
        _ => {
            eprintln!("unknown corner '{}', defaulting to top-left", corner);
            Placement {
                name: "top-left".to_string(),
                anchor: Anchor::Top | Anchor::Left,
                width: 10,
                height: 10,
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let conn = Connection::connect_to_env()?;
    let mut event_queue: EventQueue<WaylandState> = conn.new_event_queue();
    let qh = event_queue.handle();

    let placement = parse_placement(&args.corner);
    let mut state = WaylandState::new(args.command, placement.name.clone(), args.debug);

    let display = conn.display();
    let _registry = display.get_registry(&qh, ());
    event_queue.roundtrip(&mut state)?;

    if state.is_ready() {
        if args.debug {
            println!("[debug] hot corner configured: {}", placement.name);
        }
        state.create_surface(&qh, placement.anchor, placement.width, placement.height)?;
    } else {
        eprintln!("error: failed to bind required wayland globals");
    }

    loop {
        event_queue.blocking_dispatch(&mut state)?;
    }
}
