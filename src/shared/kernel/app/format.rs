pub fn time(seconds: f64) -> String {
    if !seconds.is_finite() || seconds <= 0.0 {
        return "00:00".to_owned();
    }

    let total = seconds as u64;
    format!("{:02}:{:02}", total / 60, total % 60)
}

#[cfg(test)]
#[path = "format.test.rs"]
mod tests;
