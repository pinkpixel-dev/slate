//! Raising the running window under Wayland when a second launch hands over
//! its activation token (`XDG_ACTIVATION_TOKEN`).
//!
//! GPUI only spends a token on the window it creates at startup, so this talks
//! to the compositor directly: it borrows GPUI's Wayland connection and window
//! surface through `raw-window-handle` and calls `xdg_activation_v1.activate`
//! on a private event queue.

use gpui_kit::Window;
use raw_window_handle::{HasDisplayHandle as _, HasWindowHandle, RawDisplayHandle, RawWindowHandle};
use wayland_client::backend::{Backend, ObjectId};
use wayland_client::globals::{GlobalListContents, registry_queue_init};
use wayland_client::protocol::{wl_registry, wl_surface::WlSurface};
use wayland_client::{Connection, Dispatch, EventQueue, Proxy as _, QueueHandle};
use wayland_protocols::xdg::activation::v1::client::xdg_activation_v1::{self, XdgActivationV1};

/// The compositor's activation global, bound once on our own queue.
pub struct Activator {
    connection: Connection,
    activation: XdgActivationV1,
    _queue: EventQueue<State>,
}

pub struct State;

impl Activator {
    /// `None` on X11, or when the compositor doesn't offer xdg-activation.
    pub fn new(window: &Window) -> Option<Self> {
        let RawDisplayHandle::Wayland(display) = window.display_handle().ok()?.as_raw() else {
            return None;
        };
        // SAFETY: the pointer is GPUI's live `wl_display`, which outlives every
        // window. A foreign backend never disconnects it, and libwayland allows
        // more than one event queue on a connection.
        let backend = unsafe { Backend::from_foreign_display(display.display.as_ptr().cast()) };
        let connection = Connection::from_backend(backend);
        let (globals, queue) = registry_queue_init::<State>(&connection).ok()?;
        let activation = globals.bind::<XdgActivationV1, _, _>(&queue.handle(), 1..=1, ()).ok()?;
        Some(Self {
            connection,
            activation,
            _queue: queue,
        })
    }

    /// Asks the compositor to focus `window`. It decides whether the token is
    /// still good, so this can quietly do nothing.
    pub fn activate(&self, window: &Window, token: String) {
        let Ok(handle) = HasWindowHandle::window_handle(window) else { return };
        let RawWindowHandle::Wayland(surface) = handle.as_raw() else {
            return;
        };
        // SAFETY: the pointer is the window's `wl_surface` proxy, alive while the window is.
        let Ok(id) = (unsafe { ObjectId::from_ptr(WlSurface::interface(), surface.surface.as_ptr().cast()) }) else {
            return;
        };
        let Ok(surface) = WlSurface::from_id(&self.connection, id) else {
            return;
        };
        self.activation.activate(token, &surface);
        if let Err(err) = self.connection.flush() {
            eprintln!("slate: couldn't raise the window: {err}");
        }
    }
}

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for State {
    fn event(
        _: &mut Self,
        _: &wl_registry::WlRegistry,
        _: wl_registry::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<XdgActivationV1, ()> for State {
    fn event(
        _: &mut Self,
        _: &XdgActivationV1,
        _: xdg_activation_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}
