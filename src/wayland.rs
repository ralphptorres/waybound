use memmap2::MmapMut;
use std::os::unix::io::AsFd;
use wayland_client::protocol::{wl_compositor, wl_pointer, wl_region, wl_registry, wl_seat, wl_shm, wl_surface};
use wayland_client::{Connection, Dispatch, QueueHandle, WEnum};
use wayland_protocols_wlr::layer_shell::v1::client::{zwlr_layer_shell_v1, zwlr_layer_surface_v1};

#[derive(Clone, Debug)]
pub struct HotCornerPlacement {
    pub name: String,
    pub anchor: zwlr_layer_surface_v1::Anchor,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug)]
pub struct HotCornerRule {
    pub placement: HotCornerPlacement,
    pub command: String,
}

#[derive(Clone, Debug)]
struct HotCornerSurface {
    placement_name: String,
    command: String,
    layer_surface: zwlr_layer_surface_v1::ZwlrLayerSurfaceV1,
    surface: wl_surface::WlSurface,
}

pub struct WaylandState {
    pub compositor: Option<wl_compositor::WlCompositor>,
    pub layer_shell: Option<zwlr_layer_shell_v1::ZwlrLayerShellV1>,
    pub seat: Option<wl_seat::WlSeat>,
    pub pointer: Option<wl_pointer::WlPointer>,
    pub rules: Vec<HotCornerRule>,
    surfaces: Vec<HotCornerSurface>,
    pub shm: Option<wl_shm::WlShm>,
    pub debug: bool,
}

impl WaylandState {
    pub fn new(rules: Vec<HotCornerRule>, debug: bool) -> Self {
        WaylandState {
            compositor: None,
            layer_shell: None,
            seat: None,
            pointer: None,
            rules,
            surfaces: Vec::new(),
            shm: None,
            debug,
        }
    }

    pub fn is_ready(&self) -> bool {
        self.compositor.is_some() && self.layer_shell.is_some() && self.shm.is_some()
    }

    fn hot_corner_pixel(debug: bool) -> [u8; 4] {
        if debug {
            [0x20, 0x20, 0xff, 0x66]
        } else {
            [0, 0, 0, 0]
        }
    }

    pub fn create_surfaces(&mut self, qh: &QueueHandle<Self>) -> Result<(), Box<dyn std::error::Error>> {
        let rules = self.rules.clone();
        for rule in rules {
            self.create_surface(qh, rule)?;
        }
        Ok(())
    }

    fn create_surface(
        &mut self,
        qh: &QueueHandle<Self>,
        rule: HotCornerRule,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let compositor = self.compositor.as_ref().unwrap();
        let layer_shell = self.layer_shell.as_ref().unwrap();

        let surface = compositor.create_surface(qh, ());
        let layer_surface = layer_shell.get_layer_surface(
            &surface,
            None,
            zwlr_layer_shell_v1::Layer::Overlay,
            "waybound".to_string(),
            qh,
            (),
        );

        layer_surface.set_size(rule.placement.width, rule.placement.height);
        layer_surface.set_anchor(rule.placement.anchor);

        self.surfaces.push(HotCornerSurface {
            placement_name: rule.placement.name,
            command: rule.command,
            layer_surface: layer_surface.clone(),
            surface: surface.clone(),
        });

        surface.commit();
        Ok(())
    }
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
            match interface.as_str() {
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
            for hot_corner in &state.surfaces {
                if surface == hot_corner.surface {
                    if state.debug {
                        println!(
                            "[debug] hot corner triggered: {}. executing: {}",
                            hot_corner.placement_name, hot_corner.command
                        );
                    }
                    let _ = std::process::Command::new("sh")
                        .arg("-c")
                        .arg(&hot_corner.command)
                        .spawn();
                    break;
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

            if let Some(shm) = &state.shm {
                let size = ((width * height * 4) as i32) as usize;

                let tmp_file = match tempfile::tempfile() {
                    Ok(f) => f,
                    Err(_) => return,
                };

                if tmp_file.set_len(size as u64).is_err() {
                    return;
                }

                let mut mmap = match unsafe { MmapMut::map_mut(&tmp_file) } {
                    Ok(mmap) => mmap,
                    Err(_) => return,
                };

                let pixel = Self::hot_corner_pixel(state.debug);
                mmap.chunks_exact_mut(4).for_each(|chunk| chunk.copy_from_slice(&pixel));

                if mmap.flush().is_err() {
                    return;
                }

                let pool = shm.create_pool(tmp_file.as_fd(), size as i32, qh, ());
                let buffer = pool.create_buffer(
                    0,
                    width as i32,
                    height as i32,
                    (width * 4) as i32,
                    wl_shm::Format::Argb8888,
                    qh,
                    (),
                );

                for hot_corner in &state.surfaces {
                    if hot_corner.layer_surface == *proxy {
                        hot_corner.surface.attach(Some(&buffer), 0, 0);
                        hot_corner.surface.damage_buffer(0, 0, width as i32, height as i32);
                        hot_corner.surface.commit();
                        break;
                    }
                }
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
