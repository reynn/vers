use std::error::Error;

/// Format an error and its source chain as a single concise message.
///
/// The top-level error is followed by each underlying cause separated by
/// `": "`. Empty or duplicate messages are skipped so wrappers that forward
/// their source's text do not produce repeated output.
pub fn format_error_chain(error: &(dyn Error + 'static)) -> String {
    let mut message = single_line(&error.to_string());
    if message.is_empty() {
        message.push_str("unknown error");
    }

    let mut source = error.source();
    while let Some(cause) = source {
        let cause_message = single_line(&cause.to_string());
        if !cause_message.is_empty() && !message.ends_with(&cause_message) {
            message.push_str(": ");
            message.push_str(&cause_message);
        }
        source = cause.source();
    }

    message
}

/// Collapse whitespace, including newlines, so multi-line upstream messages
/// stay on a single log line.
fn single_line(message: &'_ str) -> String {
    message.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt;

    #[derive(Debug)]
    struct TestError {
        message: &'static str,
        source: Option<Box<TestError>>,
    }

    impl TestError {
        fn new(message: &'static str) -> Self {
            Self {
                message,
                source: None,
            }
        }

        fn with_source(message: &'static str, source: TestError) -> Self {
            Self {
                message,
                source: Some(Box::new(source)),
            }
        }
    }

    impl fmt::Display for TestError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str(self.message)
        }
    }

    impl Error for TestError {
        fn source(&self) -> Option<&(dyn Error + 'static)> {
            self.source
                .as_deref()
                .map(|source| source as &(dyn Error + 'static))
        }
    }

    #[test]
    fn formats_a_single_error() {
        assert_eq!(format_error_chain(&TestError::new("boom")), "boom");
    }

    #[test]
    fn formats_the_full_source_chain() {
        let error = TestError::with_source(
            "failed to install",
            TestError::with_source("failed to download", TestError::new("connection refused")),
        );

        assert_eq!(
            format_error_chain(&error),
            "failed to install: failed to download: connection refused"
        );
    }

    #[test]
    fn skips_empty_and_duplicate_causes() {
        let error = TestError::with_source(
            "wrapped",
            TestError::with_source("wrapped", TestError::new("")),
        );

        assert_eq!(format_error_chain(&error), "wrapped");
    }

    #[test]
    fn collapses_whitespace_to_a_single_line() {
        let error = TestError::with_source(
            "failed to install\nsecond line",
            TestError::new("caused by   something"),
        );

        assert_eq!(
            format_error_chain(&error),
            "failed to install second line: caused by something"
        );
    }
}
