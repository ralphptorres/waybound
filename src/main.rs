use clap::Parser;
use wayland_client::{Connection, EventQueue};
use wayland_protocols_wlr::layer_shell::v1::client::{zwlr_layer_shell_v1, zwlr_layer_surface_v1};

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
            eprintln!("Unknown corner '{}', defaulting to top-left", corner);
            Anchor::Top | Anchor::Left
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    
    let conn = Connection::connect_to_env()?;
    let mut event_queue: EventQueue<WaylandState> = conn.new_event_queue();
    let qh = event_queue.handle();
    
    let mut state = WaylandState {
        compositor: None,
        layer_shell: None,
        seat: None,
        pointer: None,
        command: args.command,
        surface: None,
        shm: None,
        layer_surface: None,
        shm_file: None,
    };

    let display = conn.display();
    let _registry = display.get_registry(&qh, ());

    // Initial roundtrip to discover globals
    event_queue.roundtrip(&mut state)?;

    if let (Some(compositor), Some(layer_shell), Some(shm)) = (&state.compositor, &state.layer_shell, &state.shm) {
        println!("Creating hot corner surface at {}...", args.corner);
        let surface = compositor.create_surface(&qh, ());
        
        let layer_surface = layer_shell.get_layer_surface(
            &surface,
            None,
            zwlr_layer_shell_v1::Layer::Overlay,
            "waybound".to_string(),
            &qh,
            (),
        );
        
        layer_surface.set_size(10, 10);
        let anchor = parse_corner(&args.corner);
        layer_surface.set_anchor(anchor);
        
        // Create input region to receive pointer events
        let region = compositor.create_region(&qh, ());
        region.add(0, 0, 10, 10);
        surface.set_input_region(Some(&region));
        
        state.surface = Some(surface.clone());
        state.layer_surface = Some(layer_surface);
        state.shm = Some(shm.clone());
        
        // Initial commit to trigger Configure event
        surface.commit();
    } else {
        println!("Error: Failed to bind required Wayland globals");
    }

    println!("Starting main loop...");
    loop {
        event_queue.blocking_dispatch(&mut state)?;
    }
}
