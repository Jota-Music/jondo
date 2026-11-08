#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Locale {
    En,
    Es,
}

pub const AVAILABLE: [Locale; 2] = [Locale::En, Locale::Es];

impl Locale {
    pub fn code(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::Es => "es",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::En => "English",
            Self::Es => "Español",
        }
    }

    pub fn parse(code: &str) -> Option<Self> {
        let code = code.trim().to_ascii_lowercase();
        AVAILABLE
            .into_iter()
            .find(|locale| locale.code() == code || code.starts_with(locale.code()))
    }
}
