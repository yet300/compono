use crate::ScreenPaths;
use compono::Phase;
use compono::router::link::Navigation;
use compono::router::stack::Stack;
use compono_web::history::History;
use compono_web::{Address, BrowserHistory, PageEvent, PageEvents, SessionStore, WebSpawner};
use eframe::wasm_bindgen::JsCast;
use eframe::wasm_bindgen::prelude::wasm_bindgen;
use showcase::Services;
use showcase::feature::root::{Child, Root, RootMsg, Screen};
use showcase_egui::App;
use std::any::Any;
use wasm_bindgen::JsValue;

const STORE_KEY: &str = "compono-showcase";

struct Page {
    app: App,
    history: History<ScreenPaths>,
    browser: BrowserHistory,
    store: Option<SessionStore>,
    phase: Phase,
}

impl Page {
    fn new(ctx: &egui::Context) -> Result<Self, JsValue> {
        let store = SessionStore::new(STORE_KEY).ok();
        let saved = store.as_ref().and_then(SessionStore::load);
        let services = Services::new(|delay| Box::pin(compono_web::sleep(delay)));
        let app = App::new(ctx, services, WebSpawner, saved.as_deref());
        let mut page = Self {
            app,
            history: History::new(ScreenPaths, showcase::saved::SCHEMA),
            browser: BrowserHistory::new(Address::Hash)?,
            store,
            phase: compono_web::page_phase(),
        };
        let (state, path) = page.browser.current();
        let navigation =
            page.with_stack(|history, browser, stack| history.start(browser, stack, state, &path));
        page.navigate(navigation);
        Ok(page)
    }

    fn with_stack<T>(
        &mut self,
        run: impl FnOnce(
            &mut History<ScreenPaths>,
            &mut BrowserHistory,
            &Stack<Screen, Child, Root>,
        ) -> Option<T>,
    ) -> Option<T> {
        let root = self.app.tree().get(self.app.root())?;
        run(&mut self.history, &mut self.browser, &root.stack)
    }

    fn navigate(&mut self, navigation: Option<Navigation<Screen>>) {
        if let Some(Navigation { keep, open }) = navigation {
            let root = self.app.root();
            let _gone = self.app.tree_mut().send(root, RootMsg::Show { keep, open });
        }
        self.sync();
    }

    fn sync(&mut self) {
        let _nothing: Option<()> = self.with_stack(|history, browser, stack| {
            history.sync(browser, stack);
            None
        });
    }

    fn event(&mut self, event: PageEvent) {
        match event {
            PageEvent::Phase(phase) => {
                self.phase = phase;
                self.app.set_phase(phase);
                if phase == Phase::Hidden {
                    self.save();
                }
            }
            PageEvent::Popped { state, path } => {
                let navigation = self.with_stack(|history, browser, stack| {
                    history.popped(browser, stack, state, &path)
                });
                self.navigate(navigation);
            }
            // A later version may report more. This page needs nothing else.
            _ => {}
        }
    }

    fn save(&self) {
        if let (Some(store), Some(bytes)) = (&self.store, self.app.save()) {
            let _full = store.store(&bytes);
        }
    }
}

impl eframe::App for Page {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.app.frame_in(ui, self.phase);
        self.sync();
    }

    fn as_any_mut(&mut self) -> Option<&mut dyn Any> {
        Some(self)
    }
}

/// Starts the showcase on the canvas with id `showcase`.
#[wasm_bindgen(start)]
pub fn start() {
    wasm_bindgen_futures::spawn_local(async {
        if let Err(error) = run().await {
            web_sys::console::error_1(&error);
        }
    });
}

async fn run() -> Result<(), JsValue> {
    let document = web_sys::window()
        .and_then(|window| window.document())
        .ok_or_else(|| JsValue::from_str("no document"))?;
    let canvas = document
        .get_element_by_id("showcase")
        .ok_or_else(|| JsValue::from_str("no canvas with id showcase"))?
        .dyn_into::<web_sys::HtmlCanvasElement>()?;
    let runner = eframe::WebRunner::new();
    runner
        .start(
            canvas,
            eframe::WebOptions::default(),
            Box::new(|creation| {
                let page = Page::new(&creation.egui_ctx).map_err(|error| format!("{error:?}"))?;
                Ok(Box::new(page))
            }),
        )
        .await?;
    let events = runner.clone();
    PageEvents::listen(Address::Hash, move |event| {
        if let Some(mut page) = events.app_mut::<Page>() {
            page.event(event);
        }
    })?
    .forever();
    Ok(())
}
