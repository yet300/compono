//! The showcase in a desktop window. `showcase-web` runs the same app in a browser.

#[cfg(target_arch = "wasm32")]
fn main() {}

#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    desktop::run()
}

#[cfg(not(target_arch = "wasm32"))]
mod desktop {
    use compono_desktop::StateFile;
    use compono_tokio::TokioSpawner;
    use eframe::egui;
    use showcase::Services;
    use showcase_egui::App;
    use std::time::Duration;
    use tokio::runtime::Runtime;

    struct Window {
        app: App,
        state: Option<StateFile>,
        _runtime: Runtime,
    }

    impl eframe::App for Window {
        fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
            self.app.frame(ui);
        }

        fn on_exit(&mut self) {
            if let (Some(state), Some(bytes)) = (&self.state, self.app.save())
                && let Err(error) = state.store(&bytes)
            {
                eprintln!("could not save {}: {error}", state.path().display());
            }
        }
    }

    pub fn run() -> Result<(), Box<dyn std::error::Error>> {
        let runtime = Runtime::new()?;
        let handle = runtime.handle().clone();
        let sleep_handle = handle.clone();
        let services = Services::new(move |delay: Duration| {
            let _entered = sleep_handle.enter();
            Box::pin(tokio::time::sleep(delay))
        });
        let state = StateFile::in_app_dir("compono-showcase").ok();
        let saved = state.as_ref().and_then(|state| state.load().ok().flatten());
        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([1000.0, 720.0])
                .with_title("Compono showcase (egui)"),
            ..Default::default()
        };
        eframe::run_native(
            "Compono showcase",
            options,
            Box::new(move |creation| {
                let spawner = TokioSpawner::new(handle);
                let app = App::new(&creation.egui_ctx, services, spawner, saved.as_deref());
                Ok(Box::new(Window {
                    app,
                    state,
                    _runtime: runtime,
                }))
            }),
        )?;
        Ok(())
    }
}
