use wayland_client::{Display, GlobalManager};

mod wayland;
use crate::wayland::WaylandState;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let display = Display::connect_to_env()?;
    let mut event_queue = display.create_event_queue();
    let attached_display = (*display).clone().attach(event_queue.token());

    let mut state = WaylandState {
        compositor: None,
        layer_shell: None,
        surfaces: Vec::new(),
    };

    let _globals = GlobalManager::new(&attached_display);
    
    // In 0.29, we use event_queue.sync_roundtrip
    event_queue.sync_roundtrip(&mut state, |_, _, _| {})?;

    println!("Successfully connected and discovered globals.");
    Ok(())
}
