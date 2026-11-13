use compono_desktop::StateFile;
use gpui::prelude::*;
use gpui::{App, Bounds, TitlebarOptions, WindowBounds, WindowOptions, px, size};
use gpui_component::Root;
use showcase::Services;
use showcase_gpui::ShowcaseView;
use std::time::Duration;

fn main() {
    gpui_platform::application().run(|cx: &mut App| {
        gpui_component::init(cx);
        let executor = cx.background_executor().clone();
        let services = Services::new(move |delay: Duration| Box::pin(executor.timer(delay)));
        let bounds = Bounds::centered(None, size(px(1000.0), px(720.0)), cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(TitlebarOptions {
                title: Some("Compono showcase (GPUI)".into()),
                ..TitlebarOptions::default()
            }),
            ..WindowOptions::default()
        };
        let state = StateFile::in_app_dir("compono-showcase").ok();
        let saved = state.as_ref().and_then(|state| state.load().ok().flatten());
        let opened = cx.open_window(options, |window, cx| {
            let view = cx.new(|cx| {
                cx.on_release(move |view: &mut ShowcaseView, _cx| {
                    if let (Some(state), Some(bytes)) = (&state, view.save())
                        && let Err(error) = state.store(&bytes)
                    {
                        eprintln!("could not save {}: {error}", state.path().display());
                    }
                })
                .detach();
                ShowcaseView::new(window, cx, services, saved.as_deref())
            });
            // gpui-component draws tooltips and overlays from this root.
            cx.new(|cx| Root::new(view, window, cx))
        });
        if opened.is_err() {
            cx.quit();
            return;
        }
        cx.on_window_closed(|cx, _window| cx.quit()).detach();
        cx.activate(true);
    });
}
