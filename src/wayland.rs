use wayland_client::protocol::{wl_compositor, wl_surface};
use wayland_protocols::wlr::unstable::layer_shell::v1::client::zwlr_layer_shell_v1;

pub struct WaylandState {
    pub compositor: Option<wl_compositor::WlCompositor>,
    pub layer_shell: Option<zwlr_layer_shell_v1::ZwlrLayerShellV1>,
    pub surfaces: Vec<wl_surface::WlSurface>,
}
