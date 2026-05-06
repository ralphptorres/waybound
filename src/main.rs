use clap::Parser;
use wayland_client::{Connection, EventQueue};
use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1;

mod wayland;
use crate::wayland::WaylandState;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short = 'c', long)]
    command: String,

    #[arg(short = 'n', long, default_value = "top-left")]
    corner: String,
}

fn parse_corner(corner: &str) -> zwlr_layer_surface_v1::Anchor {
    use zwlr_layer_surface_v1::Anchor;
    match corner.to_lowercase().as_str() {
        "top-left" => Anchor::Top | Anchor::Left,
        "top-right" => Anchor::Top | Anchor::Right,
        "bottom-left" => Anchor::Bottom | Anchor::Left,
        "bottom-right" => Anchor::Bottom | Anchor::Right,
        "top" => Anchor::Top | Anchor::Left | Anchor::Right,
        "bottom" => Anchor::Bottom | Anchor::Left | Anchor::Right,
        "left" => Anchor::Left | Anchor::Top | Anchor::Bottom,
        "right" => Anchor::Right | Anchor::Top | Anchor::Bottom,
        _ => {
            eprintln!("unknown corner '{}', defaulting to top-left", corner);
            Anchor::Top | Anchor::Left
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let conn = Connection::connect_to_env()?;
    let mut event_queue: EventQueue<WaylandState> = conn.new_event_queue();
    let qh = event_queue.handle();

    let mut state = WaylandState::new(args.command);

    let display = conn.display();
    let _registry = display.get_registry(&qh, ());
    event_queue.roundtrip(&mut state)?;

    if state.is_ready() {
        println!("creating hot corner surface at {}...", args.corner);
        state.create_surface(&qh, parse_corner(&args.corner))?;
    } else {
        eprintln!("error: failed to bind required wayland globals");
    }

    println!("starting main loop...");
    loop {
        event_queue.blocking_dispatch(&mut state)?;
    }
}

