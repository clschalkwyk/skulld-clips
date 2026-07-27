#[derive(Debug, Clone, PartialEq)]
pub struct ProgressSnapshot {
    pub encoded_ms: u64,
    pub fps: Option<f64>,
    pub speed: Option<f64>,
    pub output_bytes: Option<u64>,
    pub terminal: bool,
}

#[derive(Debug, Default)]
pub struct ProgressParser {
    encoded_ms: u64,
    fps: Option<f64>,
    speed: Option<f64>,
    output_bytes: Option<u64>,
}

impl ProgressParser {
    pub fn push_line(&mut self, line: &str) -> Option<ProgressSnapshot> {
        let (key, value) = line.trim().split_once('=')?;
        match key {
            "out_time_us" | "out_time_ms" => {
                self.encoded_ms = value.parse::<u64>().ok()?.saturating_div(1000);
            }
            "out_time" => {
                if let Some(milliseconds) = parse_timestamp(value) {
                    self.encoded_ms = milliseconds;
                }
            }
            "fps" => self.fps = parse_nonnegative(value),
            "speed" => self.speed = parse_nonnegative(value.trim_end_matches('x')),
            "total_size" => self.output_bytes = value.parse().ok(),
            "progress" => {
                return Some(ProgressSnapshot {
                    encoded_ms: self.encoded_ms,
                    fps: self.fps,
                    speed: self.speed,
                    output_bytes: self.output_bytes,
                    terminal: value == "end",
                });
            }
            _ => {}
        }
        None
    }
}

fn parse_nonnegative(value: &str) -> Option<f64> {
    value
        .parse::<f64>()
        .ok()
        .filter(|number| number.is_finite() && *number >= 0.0)
}

fn parse_timestamp(value: &str) -> Option<u64> {
    let mut parts = value.split(':');
    let hours = parts.next()?.parse::<u64>().ok()?;
    let minutes = parts.next()?.parse::<u64>().ok()?;
    let seconds = parts.next()?.parse::<f64>().ok()?;
    if parts.next().is_some() || !seconds.is_finite() || seconds < 0.0 {
        return None;
    }
    Some(
        hours
            .saturating_mul(3_600_000)
            .saturating_add(minutes.saturating_mul(60_000))
            .saturating_add((seconds * 1000.0).round() as u64),
    )
}

#[cfg(test)]
mod tests {
    use super::ProgressParser;

    #[test]
    fn parses_ffmpeg_progress_without_exposing_raw_output() {
        let mut parser = ProgressParser::default();
        for line in [
            "frame=200",
            "fps=59.94",
            "total_size=123456",
            "out_time_us=5200000",
            "speed=1.25x",
        ] {
            assert!(parser.push_line(line).is_none());
        }
        let snapshot = parser.push_line("progress=continue").unwrap();
        assert_eq!(snapshot.encoded_ms, 5200);
        assert_eq!(snapshot.fps, Some(59.94));
        assert_eq!(snapshot.speed, Some(1.25));
        assert_eq!(snapshot.output_bytes, Some(123456));
        assert!(!snapshot.terminal);
    }

    #[test]
    fn accepts_timestamp_fallback_and_terminal_marker() {
        let mut parser = ProgressParser::default();
        parser.push_line("out_time=00:01:02.345");
        assert_eq!(parser.push_line("progress=end").unwrap().encoded_ms, 62_345);
    }
}
