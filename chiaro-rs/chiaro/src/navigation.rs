#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Screen {
    #[default]
    Dashboard,
    Settings,
    About,
}

impl Screen {
    pub fn title(self) -> &'static str {
        match self {
            Self::Dashboard => "Dashboard",
            Self::Settings => "Settings",
            Self::About => "About",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Navigation {
    current: Screen,
    previous: Option<Screen>,
}

impl Navigation {
    pub fn current(&self) -> Screen {
        self.current
    }

    pub fn previous(&self) -> Option<Screen> {
        self.previous
    }

    pub fn navigate(&mut self, page: Screen) {
        if self.current != page {
            self.previous = Some(self.current);
            self.current = page;
        }
    }

    pub fn back(&mut self) {
        if let Some(previous) = self.previous.take() {
            self.current = previous;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Navigation, Screen};

    #[test]
    fn navigation_remembers_the_previous_page() {
        let mut navigation = Navigation::default();

        navigation.navigate(Screen::Settings);

        assert_eq!(navigation.current(), Screen::Settings);
        assert_eq!(navigation.previous(), Some(Screen::Dashboard));
    }

    #[test]
    fn back_returns_to_the_previous_page() {
        let mut navigation = Navigation::default();
        navigation.navigate(Screen::About);

        navigation.back();

        assert_eq!(navigation.current(), Screen::Dashboard);
        assert_eq!(navigation.previous(), None);
    }
}
