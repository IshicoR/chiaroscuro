#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Page {
    #[default]
    Dashboard,
    Settings,
    About,
}

#[derive(Debug, Clone, Default)]
pub struct Navigation {
    current: Page,
    previous: Option<Page>,
}

impl Navigation {
    pub fn current(&self) -> Page {
        self.current
    }

    pub fn previous(&self) -> Option<Page> {
        self.previous
    }

    pub fn navigate(&mut self, page: Page) {
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
    use super::{Navigation, Page};

    #[test]
    fn navigation_remembers_the_previous_page() {
        let mut navigation = Navigation::default();

        navigation.navigate(Page::Settings);

        assert_eq!(navigation.current(), Page::Settings);
        assert_eq!(navigation.previous(), Some(Page::Dashboard));
    }

    #[test]
    fn back_returns_to_the_previous_page() {
        let mut navigation = Navigation::default();
        navigation.navigate(Page::About);

        navigation.back();

        assert_eq!(navigation.current(), Page::Dashboard);
        assert_eq!(navigation.previous(), None);
    }
}
