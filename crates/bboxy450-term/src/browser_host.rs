// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! One offscreen CEF browser. The only DevTools transport is its inherited pipes.
use bboxy450_cef::{
    cdp::Request,
    defaults::Defaults,
    engine::{self, cef},
};
use cef::{application_mac::*, *};
use objc2::{ClassType, MainThreadMarker, define_class, msg_send, runtime::Bool};
use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy, NSEvent};
use std::{
    cell::{Cell, RefCell},
    io::{self, BufRead, Write},
    path::{Path, PathBuf},
    ptr,
    sync::mpsc::{self, SyncSender},
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

define_class!(
    // This process owns the only NSApplication instance and creates it on the main thread.
    #[unsafe(super(NSApplication))]
    #[thread_kind = objc2::MainThreadOnly]
    #[name = "BBoxyBrowserApplication"]
    struct BrowserApplication;

    unsafe impl CrAppProtocol for BrowserApplication {
        #[unsafe(method(isHandlingSendEvent))]
        unsafe fn is_handling_send_event(&self) -> Bool {
            SENDING_EVENT.get()
        }
    }

    unsafe impl CrAppControlProtocol for BrowserApplication {
        #[unsafe(method(setHandlingSendEvent:))]
        unsafe fn set_handling_send_event(&self, value: Bool) {
            SENDING_EVENT.set(value);
        }
    }

    unsafe impl CefAppProtocol for BrowserApplication {}

    impl BrowserApplication {
        #[unsafe(method(sendEvent:))]
        fn send_event(&self, event: &NSEvent) {
            let previous = SENDING_EVENT.replace(Bool::YES);
            // Cocoa dispatches the event before we restore Chromium's nesting flag.
            unsafe { let _: () = msg_send![super(self), sendEvent: event]; }
            SENDING_EVENT.set(previous);
        }
    }
);

const VIEWPORT: Rect = Rect {
    x: 0,
    y: 0,
    width: 1024,
    height: 768,
};

wrap_render_handler! {
    struct Offscreen;

    impl RenderHandler {
        fn view_rect(&self, _browser: Option<&mut Browser>, rect: Option<&mut Rect>) {
            if let Some(rect) = rect {
                *rect = VIEWPORT.clone();
            }
        }

        fn screen_info(&self, _browser: Option<&mut Browser>, info: Option<&mut ScreenInfo>) -> i32 {
            let Some(info) = info else { return 0 };
            info.device_scale_factor = 1.0;
            info.depth = 24;
            info.depth_per_component = 8;
            info.rect = VIEWPORT.clone();
            info.available_rect = VIEWPORT.clone();
            1
        }
    }
}

wrap_life_span_handler! {
    struct Lifetime;

    impl LifeSpanHandler {
        fn on_before_popup(
            &self, _browser: Option<&mut Browser>, _frame: Option<&mut Frame>,
            _id: i32, _url: Option<&CefString>, _name: Option<&CefString>,
            _disposition: WindowOpenDisposition, _gesture: i32, _features: Option<&PopupFeatures>,
            _window: Option<&mut WindowInfo>, _client: Option<&mut Option<Client>>,
            _settings: Option<&mut BrowserSettings>, _extra: Option<&mut Option<DictionaryValue>>,
            _no_js: Option<&mut i32>,
        ) -> i32 {
            1
        }

        fn on_before_close(&self, _browser: Option<&mut Browser>) {
            quit_message_loop();
        }
    }
}

wrap_jsdialog_handler! {
    struct NoDialogs;

    impl JsdialogHandler {
        fn on_jsdialog(
            &self, _browser: Option<&mut Browser>, _origin: Option<&CefString>,
            _kind: JsdialogType, _message: Option<&CefString>, _default: Option<&CefString>,
            _callback: Option<&mut JsdialogCallback>, suppress: Option<&mut i32>,
        ) -> i32 {
            if let Some(suppress) = suppress {
                *suppress = 1;
            }
            0
        }

        fn on_before_unload_dialog(
            &self, _browser: Option<&mut Browser>, _message: Option<&CefString>,
            _reload: i32, callback: Option<&mut JsdialogCallback>,
        ) -> i32 {
            if let Some(callback) = callback {
                callback.cont(1, None);
            }
            1
        }
    }
}

wrap_client! {
    struct OffscreenClient;

    impl Client {
        fn render_handler(&self) -> Option<RenderHandler> {
            Some(Offscreen::new())
        }

        fn jsdialog_handler(&self) -> Option<JsdialogHandler> {
            Some(NoDialogs::new())
        }

        fn life_span_handler(&self) -> Option<LifeSpanHandler> {
            Some(Lifetime::new())
        }
    }
}

thread_local! {
    static SENDING_EVENT: Cell<Bool> = const { Cell::new(Bool::NO) };
    static HOST: RefCell<Option<BrowserHost>> = const { RefCell::new(None) };
}

wrap_task! {
    struct PipeMessage {
        request: Option<Request>,
        output: SyncSender<Vec<u8>>,
    }

    impl Task {
        fn execute(&self) {
            let host = HOST.with(|host| host.borrow().clone());
            let Some(host) = host else { return };
            match &self.request {
                None => host.close_browser(1),
                Some(request) if !engine::send(&host, request) => {
                    let error = serde_json::json!({
                        "id":request.id,"error":{"code":-32000,"message":"CEF refused CDP request"}
                    });
                    let _ = self.output.send(error.to_string().into_bytes());
                }
                Some(_) => {}
            }
        }
    }
}

pub(super) fn entry() -> Option<Result<()>> {
    let mut args = std::env::args_os().skip(1);
    if args.next().as_deref() == Some("browser-host".as_ref()) {
        return Some(match (args.next(), args.next()) {
            (Some(root), None) => run(&PathBuf::from(root)),
            _ => Err("usage: bboxy450-term browser-host PRIVATE_PROFILE_ROOT".into()),
        });
    }
    let executable = std::env::current_exe().ok()?;
    if executable
        .file_name()?
        .to_str()?
        .starts_with("bboxy450-term Helper")
        && executable
            .parent()?
            .ancestors()
            .nth(3)?
            .ends_with("Contents/Frameworks")
    {
        return Some(helper(&executable));
    }
    None
}

fn helper(executable: &Path) -> Result<()> {
    let args = cef::args::Args::new();
    let mut sandbox = cef::sandbox::Sandbox::new();
    sandbox.initialize(args.as_main_args());
    let loader = cef::library_loader::LibraryLoader::new(executable, true);
    if !loader.load() {
        return Err("could not load bundled Chromium in helper".into());
    }
    let _ = api_hash(sys::CEF_API_VERSION_LAST, 0);
    let code = execute_process(
        Some(args.as_main_args()),
        Some(&mut engine::app()),
        ptr::null_mut(),
    );
    std::process::exit(if code < 0 { 1 } else { code });
}

fn run(root: &Path) -> Result<()> {
    if !root.is_absolute() || !root.is_dir() {
        return Err("browser-host requires an existing absolute private profile root".into());
    }
    let executable = std::env::current_exe()?;
    let framework = executable
        .parent()
        .ok_or("browser executable has no parent")?
        .join("../Frameworks/Chromium Embedded Framework.framework/Chromium Embedded Framework");
    if !framework.is_file() {
        return Err("browser-host requires the bundled Chromium framework".into());
    }
    let loader = cef::library_loader::LibraryLoader::new(&executable, false);
    if !loader.load() {
        return Err("could not load bundled Chromium".into());
    }
    let _ = api_hash(sys::CEF_API_VERSION_LAST, 0);
    let _main = MainThreadMarker::new().ok_or("CEF requires the main thread")?;
    // Like NSApplication::sharedApplication, with our required CefAppProtocol subclass.
    let app: objc2::rc::Retained<NSApplication> =
        unsafe { msg_send![BrowserApplication::class(), sharedApplication] };
    app.setActivationPolicy(NSApplicationActivationPolicy::Prohibited);

    let args = cef::args::Args::new();
    let mut application = engine::app();
    let settings = engine::settings(&Defaults::new(root));
    if initialize(
        Some(args.as_main_args()),
        Some(&settings),
        Some(&mut application),
        ptr::null_mut(),
    ) == 0
    {
        return Err(format!("CEF initialization failed ({})", get_exit_code()).into());
    }
    let result = serve();
    shutdown();
    result
}

fn serve() -> Result<()> {
    let window = WindowInfo::default().set_as_windowless(ptr::null_mut());
    let mut client = OffscreenClient::new();
    let browser = browser_host_create_browser_sync(
        Some(&window),
        Some(&mut client),
        Some(&CefString::from("about:blank")),
        Some(&BrowserSettings::default()),
        None,
        None,
    )
    .ok_or("CEF could not create the offscreen browser")?;
    let host = browser.host().ok_or("CEF browser has no host")?;
    let (send, receive) = mpsc::sync_channel::<Vec<u8>>(32);
    let registration =
        host.add_dev_tools_message_observer(Some(&mut engine::observer(send.clone())));
    HOST.with(|cell| cell.replace(Some(host)));
    drop(browser);

    let writer = std::thread::spawn(move || -> io::Result<()> {
        let mut out = io::stdout().lock();
        writeln!(out, r#"{{"method":"BBoxy.browserReady"}}"#)?;
        out.flush()?;
        while let Ok(line) = receive.recv() {
            out.write_all(&line)?;
            out.write_all(b"\n")?;
            out.flush()?;
        }
        Ok(())
    });

    std::thread::spawn(move || {
        let mut stdin = io::stdin().lock();
        loop {
            let mut line = Vec::new();
            // The seat MCP argument limit is 64 KiB. Leave room for our CDP envelope.
            let mut limited = io::Read::take(&mut stdin, 128 * 1024);
            if !limited.read_until(b'\n', &mut line).is_ok_and(|n| n > 0)
                || line.pop() != Some(b'\n')
            {
                break;
            }
            let Ok(value) = serde_json::from_slice::<serde_json::Value>(&line) else {
                break;
            };
            let Some(id) = value["id"].as_u64() else {
                break;
            };
            let request = Request { id, bytes: line };
            if post_task(
                ThreadId::UI,
                Some(&mut PipeMessage::new(Some(request), send.clone())),
            ) == 0
            {
                return;
            }
        }
        post_task(ThreadId::UI, Some(&mut PipeMessage::new(None, send)));
    });

    run_message_loop();
    HOST.with(|cell| cell.take());
    drop(registration);
    // Shutdown retires any task/observer holding a sender. Do not join the
    // writer before then: Chromium may still own a DevTools callback.
    drop(writer);
    Ok(())
}
