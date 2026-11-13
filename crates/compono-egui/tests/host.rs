//! `EguiHost` under the host conformance suite.

use compono::{Component, Cx, Tree};
use compono_egui::EguiHost;
use compono_test::check_host;
use egui::Context;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[test]
fn egui_host_conforms() {
    let requests = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&requests);
    let reader = Arc::clone(&requests);
    let result = check_host(
        move || {
            let ctx = Context::default();
            let counter = Arc::clone(&counter);
            ctx.set_request_repaint_callback(move |_info| {
                counter.fetch_add(1, Ordering::SeqCst);
            });
            EguiHost::new(ctx)
        },
        move || reader.load(Ordering::SeqCst),
    );
    assert_eq!(result, Ok(()));
}

struct Counter(u32);
impl Component for Counter {
    type Msg = ();
    type Output = ();
    fn update(&mut self, (): (), _cx: &mut Cx<'_, Self>) {
        self.0 += 1;
    }
}

#[test]
fn changes_made_before_drawing_ask_for_no_extra_frame() {
    let ctx = Context::default();
    let frames = |ctx: &Context| {
        let mut output = ctx.run_ui(egui::RawInput::default(), |_ui| {});
        output.textures_delta.clear();
    };
    let host = EguiHost::new(ctx.clone());
    let before_draw = host.before_draw();
    let mut tree = Tree::new(host);
    let counter = tree.mount(|_cx| Counter(0));
    // egui asks for a few frames of its own while starting up.
    for _ in 0..10 {
        frames(&ctx);
    }
    assert!(!ctx.has_requested_repaint());

    assert_eq!(before_draw.run(|| tree.send(counter, ())), Ok(()));
    assert!(!ctx.has_requested_repaint());

    assert_eq!(tree.send(counter, ()), Ok(()));
    assert!(ctx.has_requested_repaint());
}
