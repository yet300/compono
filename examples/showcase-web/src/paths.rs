use compono::router::link::Paths;
use showcase::feature::root::Screen;

/// Addresses of the root screens. Every screen opens above the tabs.
#[derive(Clone, Copy, Debug, Default)]
pub struct ScreenPaths;

impl Paths for ScreenPaths {
    type Route = Screen;

    fn path(&self, screen: &Screen) -> String {
        match screen {
            Screen::Tabs => "/",
            Screen::Book => "/book",
            Screen::Gallery => "/gallery",
        }
        .to_owned()
    }

    fn routes(&self, path: &str) -> Vec<Screen> {
        match path.trim_end_matches('/') {
            "" => vec![Screen::Tabs],
            "/book" => vec![Screen::Tabs, Screen::Book],
            "/gallery" => vec![Screen::Tabs, Screen::Gallery],
            _ => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ScreenPaths;
    use compono::router::link::Paths;
    use showcase::feature::root::Screen;

    #[test]
    fn every_screen_reads_back_from_its_path() {
        for screen in [Screen::Tabs, Screen::Book, Screen::Gallery] {
            let path = ScreenPaths.path(&screen);
            assert_eq!(ScreenPaths.routes(&path).last(), Some(&screen));
        }
        assert_eq!(ScreenPaths.routes("/book/"), [Screen::Tabs, Screen::Book]);
        assert!(ScreenPaths.routes("/nowhere").is_empty());
    }
}
