/// Android cannot read a browser jar (one WebView, no profile on disk), so the
/// panel offers the pasted cookies only.
pub fn available() -> Vec<&'static str> {
    Vec::new()
}

pub fn cookies(_name: &str) -> Result<String, String> {
    Err("this platform cannot read a browser session".to_owned())
}
