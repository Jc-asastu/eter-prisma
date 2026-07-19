//! Webview editor para nih-plug — fork ÉTER.
//! Portado a baseview actual (rwh 0.6, WindowHandler por &self, WindowContext)
//! con puente HWND crudo hacia wry 0.35 (rwh 0.5). Windows-first.

use baseview::dpi::{LogicalSize, Size};
use baseview::{Event, EventStatus, Window, WindowContext, WindowHandle as BvWindowHandle,
               WindowOpenOptions, WindowScalePolicy, WindowSize};
use nih_plug::prelude::{Editor, GuiContext, ParamSetter, ParentWindowHandle};
use serde_json::Value;
use std::{
    borrow::Cow,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
};
use wry::{
    http::{Request, Response},
    WebContext, WebView, WebViewBuilder,
};

use crossbeam::channel::{unbounded, Receiver};

pub use keyboard_types::*;
pub use wry::http;

pub use baseview::MouseEvent;

type EventLoopHandler = dyn Fn(&WindowHandler, ParamSetter) + Send + Sync;
type KeyboardHandler = dyn Fn(KeyboardEvent) -> bool + Send + Sync;
type MouseHandler = dyn Fn(MouseEvent) -> EventStatus + Send + Sync;
type CustomProtocolHandler =
    dyn Fn(&Request<Vec<u8>>) -> wry::Result<Response<Cow<'static, [u8]>>> + Send + Sync;

pub struct WebViewEditor {
    source: Arc<HTMLSource>,
    width: Arc<AtomicU32>,
    height: Arc<AtomicU32>,
    event_loop_handler: Arc<EventLoopHandler>,
    keyboard_handler: Arc<KeyboardHandler>,
    mouse_handler: Arc<MouseHandler>,
    custom_protocol: Option<(String, Arc<CustomProtocolHandler>)>,
    developer_mode: bool,
    background_color: (u8, u8, u8, u8),
}

pub enum HTMLSource {
    String(&'static str),
    URL(&'static str),
}

impl WebViewEditor {
    pub fn new(source: HTMLSource, size: (u32, u32)) -> Self {
        Self {
            source: Arc::new(source),
            width: Arc::new(AtomicU32::new(size.0)),
            height: Arc::new(AtomicU32::new(size.1)),
            developer_mode: false,
            background_color: (255, 255, 255, 255),
            event_loop_handler: Arc::new(|_, _| {}),
            keyboard_handler: Arc::new(|_| false),
            mouse_handler: Arc::new(|_| EventStatus::Ignored),
            custom_protocol: None,
        }
    }

    pub fn with_background_color(mut self, background_color: (u8, u8, u8, u8)) -> Self {
        self.background_color = background_color;
        self
    }

    pub fn with_custom_protocol<F>(mut self, name: String, handler: F) -> Self
    where
        F: Fn(&Request<Vec<u8>>) -> wry::Result<Response<Cow<'static, [u8]>>>
            + 'static
            + Send
            + Sync,
    {
        self.custom_protocol = Some((name, Arc::new(handler)));
        self
    }

    pub fn with_event_loop<F>(mut self, handler: F) -> Self
    where
        F: Fn(&WindowHandler, ParamSetter) + 'static + Send + Sync,
    {
        self.event_loop_handler = Arc::new(handler);
        self
    }

    pub fn with_developer_mode(mut self, mode: bool) -> Self {
        self.developer_mode = mode;
        self
    }

    pub fn with_keyboard_handler<F>(mut self, handler: F) -> Self
    where
        F: Fn(KeyboardEvent) -> bool + Send + Sync + 'static,
    {
        self.keyboard_handler = Arc::new(handler);
        self
    }

    pub fn with_mouse_handler<F>(mut self, handler: F) -> Self
    where
        F: Fn(MouseEvent) -> EventStatus + Send + Sync + 'static,
    {
        self.mouse_handler = Arc::new(handler);
        self
    }
}

pub struct WindowHandler {
    context: Arc<dyn GuiContext>,
    window_ctx: WindowContext,
    event_loop_handler: Arc<EventLoopHandler>,
    keyboard_handler: Arc<KeyboardHandler>,
    mouse_handler: Arc<MouseHandler>,
    webview: Option<WebView>,
    events_receiver: Receiver<Value>,
    pub width: Arc<AtomicU32>,
    pub height: Arc<AtomicU32>,
}

impl WindowHandler {
    /// Redimensiona ventana + webview (pedido desde el event loop del plugin).
    pub fn resize(&self, width: u32, height: u32) {
        if let Some(wv) = &self.webview {
            wv.set_bounds(wry::Rect { x: 0, y: 0, width, height });
        }
        self.width.store(width, Ordering::Relaxed);
        self.height.store(height, Ordering::Relaxed);
        self.context.request_resize();
        self.window_ctx
            .resize(Size::Logical(LogicalSize::new(width as f64, height as f64)));
    }

    pub fn send_json(&self, json: Value) {
        let json_str = json.to_string();
        let quoted = serde_json::to_string(&json_str).expect("string siempre serializa");
        if let Some(wv) = &self.webview {
            wv.evaluate_script(&format!("onPluginMessageInternal({});", quoted))
                .ok();
        }
    }

    pub fn next_event(&self) -> Result<Value, crossbeam::channel::TryRecvError> {
        self.events_receiver.try_recv()
    }
}

impl baseview::WindowHandler for WindowHandler {
    fn on_frame(&self) {
        let setter = ParamSetter::new(&*self.context);
        (self.event_loop_handler)(self, setter);
    }

    fn resized(&self, size: WindowSize) {
        let w = size.logical.width.round() as u32;
        let h = size.logical.height.round() as u32;
        if let Some(wv) = &self.webview {
            wv.set_bounds(wry::Rect { x: 0, y: 0, width: w, height: h });
        }
        self.width.store(w, Ordering::Relaxed);
        self.height.store(h, Ordering::Relaxed);
    }

    fn on_event(&self, event: Event) -> EventStatus {
        match event {
            Event::Keyboard(event) => {
                if (self.keyboard_handler)(event) {
                    EventStatus::Captured
                } else {
                    EventStatus::Ignored
                }
            }
            Event::Mouse(mouse_event) => (self.mouse_handler)(mouse_event),
            _ => EventStatus::Ignored,
        }
    }
}

/// Puente: ParentWindowHandle de nih-plug → HasWindowHandle de rwh 0.6.
struct ParentRwh06(isize);

impl rwh06::HasWindowHandle for ParentRwh06 {
    fn window_handle(&self) -> Result<rwh06::WindowHandle<'_>, rwh06::HandleError> {
        let hwnd =
            std::num::NonZeroIsize::new(self.0).ok_or(rwh06::HandleError::Unavailable)?;
        let raw = rwh06::RawWindowHandle::Win32(rwh06::Win32WindowHandle::new(hwnd));
        // SAFETY: el HWND vive lo que el editor del host (contrato de nih-plug).
        unsafe { Ok(rwh06::WindowHandle::borrow_raw(raw)) }
    }
}

/// Puente: HWND crudo → HasRawWindowHandle de rwh 0.5 (lo que espera wry 0.35).
struct Hwnd05(isize);

unsafe impl rwh05::HasRawWindowHandle for Hwnd05 {
    fn raw_window_handle(&self) -> rwh05::RawWindowHandle {
        let mut h = rwh05::Win32WindowHandle::empty();
        h.hwnd = self.0 as *mut std::ffi::c_void;
        rwh05::RawWindowHandle::Win32(h)
    }
}

struct Instance {
    window_handle: BvWindowHandle,
}

impl Drop for Instance {
    fn drop(&mut self) {
        self.window_handle.close();
    }
}

unsafe impl Send for Instance {}

impl Editor for WebViewEditor {
    fn spawn(
        &self,
        parent: ParentWindowHandle,
        context: Arc<dyn GuiContext>,
    ) -> Box<dyn std::any::Any + Send> {
        let parent_hwnd = match parent {
            ParentWindowHandle::Win32Hwnd(hwnd) => hwnd as isize,
            _ => panic!("nih_plug_webview (fork ÉTER): solo Windows por ahora"),
        };

        let options = WindowOpenOptions::new()
            .with_title("PRISMA")
            .with_size(LogicalSize::new(
                self.width.load(Ordering::Relaxed) as f64,
                self.height.load(Ordering::Relaxed) as f64,
            ))
            .with_scale_policy(WindowScalePolicy::SystemScaleFactor);

        let width = self.width.clone();
        let height = self.height.clone();
        let developer_mode = self.developer_mode;
        let source = self.source.clone();
        let background_color = self.background_color;
        let custom_protocol = self.custom_protocol.clone();
        let event_loop_handler = self.event_loop_handler.clone();
        let keyboard_handler = self.keyboard_handler.clone();
        let mouse_handler = self.mouse_handler.clone();

        let window_handle = Window::open_parented(
            &ParentRwh06(parent_hwnd),
            options,
            move |window: WindowContext| {
                let (events_sender, events_receiver) = unbounded();

                // HWND de la ventana hija recién creada (rwh 0.6 → crudo)
                let child_hwnd = match rwh06::HasWindowHandle::window_handle(&window)
                    .expect("window handle")
                    .as_raw()
                {
                    rwh06::RawWindowHandle::Win32(h) => h.hwnd.get(),
                    _ => panic!("nih_plug_webview (fork ÉTER): solo Windows por ahora"),
                };
                let child = Hwnd05(child_hwnd);

                // user-data-folder propio por proceso: compartir el temp dir global
                // colisiona con el environment WebView2 del host (HRESULT 0x8007139F)
                let data_dir = std::env::temp_dir()
                    .join(format!("eter-prisma-webview-{}", std::process::id()));
                let mut web_context = WebContext::new(Some(data_dir));

                let mut webview_builder = WebViewBuilder::new_as_child(&child)
                    .with_bounds(wry::Rect {
                        x: 0,
                        y: 0,
                        width: width.load(Ordering::Relaxed),
                        height: height.load(Ordering::Relaxed),
                    })
                    .with_accept_first_mouse(true)
                    .with_devtools(developer_mode)
                    .with_web_context(&mut web_context)
                    .with_initialization_script(include_str!("script.js"))
                    .with_ipc_handler(move |msg: String| {
                        if let Ok(json_value) = serde_json::from_str(&msg) {
                            let _ = events_sender.send(json_value);
                        }
                    })
                    .with_background_color(background_color);

                if let Some(custom_protocol) = custom_protocol.as_ref() {
                    let handler = custom_protocol.1.clone();
                    webview_builder = webview_builder
                        .with_custom_protocol(custom_protocol.0.to_owned(), move |request| {
                            handler(&request).unwrap()
                        });
                }

                // Nunca abortar el host: si el webview no se puede construir
                // (runtime WebView2 ausente/roto), la ventana queda vacía y el
                // audio sigue funcionando.
                let webview = match source.as_ref() {
                    HTMLSource::String(html_str) => webview_builder.with_html(*html_str),
                    HTMLSource::URL(url) => webview_builder.with_url(*url),
                }
                .and_then(|b| b.build())
                .map_err(|e| eprintln!("eter-prisma: fallo el webview del editor: {e}"))
                .ok();

                WindowHandler {
                    context,
                    window_ctx: window,
                    event_loop_handler,
                    webview,
                    events_receiver,
                    keyboard_handler,
                    mouse_handler,
                    width,
                    height,
                }
            },
        );

        Box::new(Instance { window_handle })
    }

    fn size(&self) -> (u32, u32) {
        (
            self.width.load(Ordering::Relaxed),
            self.height.load(Ordering::Relaxed),
        )
    }

    fn set_scale_factor(&self, _factor: f32) -> bool {
        false
    }

    fn param_values_changed(&self) {}

    fn param_value_changed(&self, _id: &str, _normalized_value: f32) {}

    fn param_modulation_changed(&self, _id: &str, _modulation_offset: f32) {}
}
