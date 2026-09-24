#![forbid(unsafe_code)]

//! The regex route technology — a technology of `xmip-core-route`.
//!
//! A Subscription's filter names properties, and each property is read from
//! one source. This source extracts a capture from any text value:
//! `regex:<property>:<pattern>` applies `<pattern>` to the text of the context
//! value `<property>` and reads the first capture group, or the whole match
//! where the pattern has no group — `regex:OrderNo:^INV-(\d+)$` reads the
//! digits of an order number, `regex:Subject:urgent|asap` reads whichever word
//! is there. The pattern is everything after the second colon, so it may hold
//! colons of its own. A property the context does not hold, a `Null`, and a
//! pattern that does not match promote nothing, so a filter over them declines
//! with its reason. A pattern that does not compile, bytes, and a property with
//! no pattern are errors. ADR-0046.
//!
//! The text is the value rendered as a filter would compare it — text as it
//! is, a boolean as `true` or `false`, a number as it prints — so a pattern
//! can read a digit out of an integer as easily as out of a string. It is read
//! through `route::routable`, as every context value a filter names is
//! (ADR-0046, amended 2026-09-24).
//!
//! A route technology does not decide anything: it reads.

use message::Message;
use regex::Regex;
use route::{Source, SourceError};

/// The manifest leaf and the prefix a property carries.
pub const TECHNOLOGY: &str = "regex";

/// Reads `regex:<property>:<pattern>` as the first capture, or the match.
pub struct RegexSource;

impl Source for RegexSource {
    fn technology(&self) -> &'static str {
        TECHNOLOGY
    }

    fn read(&self, message: &Message, name: &str) -> Result<Option<String>, SourceError> {
        let refuse = |reason: String| SourceError::new(TECHNOLOGY, name, reason);

        let Some((property, pattern)) = name.split_once(':') else {
            return Err(refuse(
                "a property is regex:<property>:<pattern>".to_string(),
            ));
        };
        if property.is_empty() || pattern.is_empty() {
            return Err(refuse(
                "both a property and a pattern are needed".to_string(),
            ));
        }

        let regex = Regex::new(pattern).map_err(|error| refuse(error.to_string()))?;

        let Some(text) =
            route::routable(property, message.context().get(property)).map_err(refuse)?
        else {
            return Ok(None);
        };

        Ok(regex.captures(&text).map(|captures| {
            captures
                .get(1)
                .or_else(|| captures.get(0))
                .map_or_else(String::new, |found| found.as_str().to_string())
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use context::{ContextValue, MessageContext};
    use message::MessageTreatment;
    use route::{Predicate, Value};
    use xcore::MessageId;

    fn message() -> Message {
        let context = MessageContext::new()
            .with_value("OrderNo", ContextValue::Text("INV-0012345".into()))
            .with_value("Subject", ContextValue::Text("Please treat as ASAP".into()))
            .with_value("When", ContextValue::Text("2026-09-10T09:30:00Z".into()))
            .with_value("Amount", ContextValue::Integer(1500))
            .with_value("Note", ContextValue::Null)
            .with_value("Blob", ContextValue::Binary(vec![0, 1]));
        Message::received(
            MessageId::new(1),
            Vec::new(),
            context,
            MessageTreatment::default(),
        )
    }

    fn read(name: &str) -> Result<Option<String>, SourceError> {
        RegexSource.read(&message(), name)
    }

    #[test]
    fn the_first_capture_is_read_and_the_whole_match_where_there_is_none() {
        assert_eq!(
            read(r"OrderNo:^INV-(\d+)$").expect("capture"),
            Some("0012345".into())
        );
        assert_eq!(
            read("Subject:(?i)urgent|asap").expect("match"),
            Some("ASAP".into())
        );
        assert_eq!(
            read(r"Amount:\d\d$").expect("integer as text"),
            Some("00".into())
        );
    }

    #[test]
    fn a_pattern_may_hold_colons_of_its_own() {
        assert_eq!(
            read(r"When:T(\d\d:\d\d):").expect("time"),
            Some("09:30".into())
        );
    }

    #[test]
    fn no_match_an_absent_property_and_a_null_promote_nothing() {
        assert_eq!(read(r"OrderNo:^PO-(\d+)$").expect("no match"), None);
        assert_eq!(read(r"Region:.*").expect("absent"), None);
        assert_eq!(read(r"Note:.*").expect("null"), None);
    }

    #[test]
    fn a_pattern_that_does_not_compile_bytes_and_a_missing_pattern_are_refused() {
        let broken = read("OrderNo:(unclosed").expect_err("does not compile");
        assert_eq!(broken.technology, "regex");
        assert_eq!(broken.property, "OrderNo:(unclosed");
        assert!(broken.reason.contains("regex parse error"));

        let bytes = read("Blob:.").expect_err("bytes");
        assert!(bytes.reason.contains("Blob holds 2 bytes"));

        let shape = read("OrderNo").expect_err("no pattern");
        assert!(shape.reason.contains("regex:<property>:<pattern>"));
        assert!(read(":x").is_err());
        assert!(read("OrderNo:").is_err());
    }

    #[test]
    fn the_technology_is_regex_and_promote_reads_the_prefixed_property() {
        assert_eq!(RegexSource.technology(), "regex");

        let sources: [&dyn Source; 1] = [&RegexSource];
        let number = r"regex:OrderNo:^INV-(\d+)$";
        let promoted =
            route::promote(&message(), &sources, &[number, "regex:Note:.*"]).expect("readable");

        assert_eq!(promoted.get(number), Some("0012345"));
        assert_eq!(promoted.get("regex:Note:.*"), None);
        assert!(
            Predicate::equals(number, Value::Integer(12345))
                .test(&promoted)
                .passed()
        );
        assert!(
            Predicate::equals(number, Value::Text("0012345".into()))
                .test(&promoted)
                .passed()
        );
    }
}
