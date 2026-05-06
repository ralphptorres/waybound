use wayland_client::protocol::{wl_compositor, wl_pointer, wl_region, wl_registry, wl_seat, wl_surface, wl_shm};
use wayland_protocols_wlr::layer_shell::v1::client::{zwlr_layer_shell_v1, zwlr_layer_surface_v1};
use wayland_client::{Dispatch, Connection, QueueHandle, WEnum};
use std::os::unix::io::AsRawFd;

pub struct WaylandState {
    pub compositor: Option<wl_compositor::WlCompositor>,
    pub layer_shell: Option<zwlr_layer_shell_v1::ZwlrLayerShellV1>,
    pub seat: Option<wl_seat::WlSeat>,
    pub pointer: Option<wl_pointer::WlPointer>,
    pub command: String,
    pub surface: Option<wl_surface::WlSurface>,
    pub shm: Option<wl_shm::WlShm>,
    pub layer_surface: Option<zwlr_layer_surface_v1::ZwlrLayerSurfaceV1>,
    pub shm_file: Option<std::fs::File>,
}

impl Dispatch<wl_registry::WlRegistry, ()> for WaylandState {
    fn event(
        state: &mut Self,
        proxy: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global { name, interface, version } = event {
            match &interface[..] {
                "wl_compositor" => {
                    state.compositor = Some(proxy.bind::<wl_compositor::WlCompositor, _, _>(name, version, qh, ()));
                }
                "zwlr_layer_shell_v1" => {
                    state.layer_shell = Some(proxy.bind::<zwlr_layer_shell_v1::ZwlrLayerShellV1, _, _>(name, version, qh, ()));
                }
                "wl_seat" => {
                    state.seat = Some(proxy.bind::<wl_seat::WlSeat, _, _>(name, version, qh, ()));
                }
                "wl_shm" => {
                    state.shm = Some(proxy.bind::<wl_shm::WlShm, _, _>(name, version, qh, ()));
                }
                _ => {}
            }
        }
    }
}

impl Dispatch<wl_seat::WlSeat, ()> for WaylandState {
    fn event(state: &mut Self, proxy: &wl_seat::WlSeat, event: wl_seat::Event, _: &(), _: &Connection, qh: &QueueHandle<Self>) {
        if let wl_seat::Event::Capabilities { capabilities: WEnum::Value(capabilities) } = event {
            if capabilities.contains(wl_seat::Capability::Pointer) {
                state.pointer = Some(proxy.get_pointer(qh, ()));
            }
        }
    }
}

impl Dispatch<wl_pointer::WlPointer, ()> for WaylandState {
    fn event(state: &mut Self, _proxy: &wl_pointer::WlPointer, event: wl_pointer::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {
        if let wl_pointer::Event::Enter { surface, .. } = event {
            if let Some(ref my_surface) = state.surface {
                if &surface == my_surface {
                    println!("Pointer entered hot corner! Executing: {}", state.command);
                    std::process::Command::new("sh").arg("-c").arg(&state.command).spawn().ok();
                }
            }
        }
    }
}

impl Dispatch<wl_compositor::WlCompositor, ()> for WaylandState {
    fn event(_: &mut Self, _: &wl_compositor::WlCompositor, _: wl_compositor::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}

impl Dispatch<zwlr_layer_shell_v1::ZwlrLayerShellV1, ()> for WaylandState {
    fn event(_: &mut Self, _: &zwlr_layer_shell_v1::ZwlrLayerShellV1, _: zwlr_layer_shell_v1::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}

impl Dispatch<zwlr_layer_surface_v1::ZwlrLayerSurfaceV1, ()> for WaylandState {
    fn event(state: &mut Self, proxy: &zwlr_layer_surface_v1::ZwlrLayerSurfaceV1, event: zwlr_layer_surface_v1::Event, _: &(), _: &Connection, qh: &QueueHandle<Self>) {
        if let zwlr_layer_surface_v1::Event::Configure { serial, width, height } = event {
            proxy.ack_configure(serial);
            
            if let (Some(shm), Some(ref surface)) = (&state.shm, &state.surface) {
                let size = ((width * height * 4) as i32) as usize;
                
                let tmp_file = match tempfile::tempfile() {
                    Ok(f) => f,
                    Err(_) => return,
                };
                
                if tmp_file.set_len(size as u64).is_err() {
                    return;
                }
                
                let fd = tmp_file.as_raw_fd();
                unsafe {
                    let borrowed_fd = std::os::unix::io::BorrowedFd::borrow_raw(fd);
                    let pool = shm.create_pool(borrowed_fd, size as i32, qh, ());
                    let buffer = pool.create_buffer(
                        0, 
                        width as i32, 
                        height as i32, 
                        (width * 4) as i32, 
                        wl_shm::Format::Argb8888, 
                        qh, 
                        ()
                    );
                    
                    surface.attach(Some(&buffer), 0, 0);
                    surface.damage_buffer(0, 0, width as i32, height as i32);
                    surface.commit();
                }
                
                state.shm_file = Some(tmp_file);
            }
        }
    }
}

impl Dispatch<wl_surface::WlSurface, ()> for WaylandState {
    fn event(_: &mut Self, _: &wl_surface::WlSurface, _: wl_surface::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}

impl Dispatch<wl_region::WlRegion, ()> for WaylandState {
    fn event(_: &mut Self, _: &wl_region::WlRegion, _: wl_region::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}

impl Dispatch<wl_shm::WlShm, ()> for WaylandState {
    fn event(_: &mut Self, _: &wl_shm::WlShm, _: wl_shm::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}

impl Dispatch<wayland_client::protocol::wl_buffer::WlBuffer, ()> for WaylandState {
    fn event(_: &mut Self, _: &wayland_client::protocol::wl_buffer::WlBuffer, _: wayland_client::protocol::wl_buffer::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}

impl Dispatch<wayland_client::protocol::wl_shm_pool::WlShmPool, ()> for WaylandState {
    fn event(_: &mut Self, _: &wayland_client::protocol::wl_shm_pool::WlShmPool, _: wayland_client::protocol::wl_shm_pool::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}
