/// Android cannot read a browser jar (one WebView, no profile on disk) and the
/// other platforms keep their cookies where this build cannot reach them, so the
/// panel offers the pasted cookies only.
pub fn available() -> Vec<&'static str> {
    Vec::new()
}

pub fn cookies(_name: &str) -> Result<String, String> {
    Err("this platform cannot read a browser session".to_owned())
}
