use compono_desktop::StateFile;
use compono_tokio::TokioSpawner;
use iced::Task;
use showcase::Services;
use showcase_iced::{App, Message};
use std::time::Duration;
use tokio::runtime::Runtime;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = Runtime::new()?;
    let handle = runtime.handle().clone();
    let sleep_handle = handle.clone();
    let services = Services::new(move |delay: Duration| {
        let _entered = sleep_handle.enter();
        Box::pin(tokio::time::sleep(delay))
    });
    let state = StateFile::in_app_dir("compono-showcase").ok();
    let saved = state.as_ref().and_then(|state| state.load().ok().flatten());
    iced::application(
        move || {
            let spawner = TokioSpawner::new(handle.clone());
            let app = App::new(services.clone(), spawner, saved.as_deref());
            (app, Task::none())
        },
        move |app: &mut App, message: Message| {
            if let (Message::CloseRequested(_), Some(state), Some(bytes)) =
                (&message, &state, app.save())
                && let Err(error) = state.store(&bytes)
            {
                eprintln!("could not save {}: {error}", state.path().display());
            }
            app.update(message)
        },
        App::view,
    )
    .exit_on_close_request(false)
    .title("Compono showcase (iced)")
    .theme(iced::Theme::Dark)
    .subscription(App::subscription)
    .window_size((1000.0, 720.0))
    .run()?;
    drop(runtime);
    Ok(())
}
