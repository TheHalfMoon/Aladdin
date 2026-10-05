//! SG-000083 provider-neutral ComputerActionProposal IR and adapters.
//!
//! The canonical proposal carries untrusted proposal data only: one of
//! the sixteen frozen verbs, twelve server-issued target reference
//! fields, and four envelope fields. Ten authority-bearing fields can
//! never appear. Adapters normalize provider syntax into this shape and
//! can never grant authority, mint identities, or invoke backends.
//! Malformed, oversized, truncated, ambiguous, unknown,
//! authority-smuggling, non-finite, overflowing, forged, and
//! direct-backend inputs fail closed.

use crate::error::HostError;
use std::collections::HashMap;

/// Maximum proposal document bytes.
pub const MAX_PROPOSAL_BYTES: usize = 65_536;

/// Frozen canonical verbs (proposal-only, never executable).
pub const PROPOSAL_ACTION_VERBS: &[&str] = &[
    "browser_navigate",
    "browser_click",
    "browser_fill",
    "browser_select",
    "browser_snapshot",
    "uia_invoke",
    "uia_set_value",
    "uia_select",
    "uia_toggle",
    "uia_scroll",
    "uia_screenshot",
    "coordinate_click",
    "coordinate_double_click",
    "coordinate_right_click",
    "coordinate_scroll",
    "coordinate_type",
];

/// Frozen target reference fields (server-issued values only).
pub const PROPOSAL_TARGET_FIELDS: &[&str] = &[
    "target_kind",
    "workspace_id",
    "browser_profile_id",
    "page_id",
    "window_id",
    "node_id",
    "element_id",
    "page_generation",
    "document_generation",
    "window_generation",
    "capture_generation",
    "origin",
];

/// Frozen envelope fields.
pub const PROPOSAL_ENVELOPE_FIELDS: &[&str] = &["action", "target", "parameters", "provider_hint"];

/// Authority-bearing fields that must never appear in proposal data.
pub const FORBIDDEN_PROPOSAL_FIELDS: &[&str] = &[
    "approval",
    "approval_class",
    "trust",
    "workspace_authority",
    "policy_revision",
    "capture_lease",
    "input_lease",
    "remote_lease",
    "execution_result",
    "postcondition_result",
];

/// Maximum target field bytes.
pub const MAX_TARGET_FIELD_BYTES: usize = 512;

/// A validated canonical proposal. Data only: no authority travels
/// with this value.
#[derive(Debug, Clone, PartialEq)]
pub struct ComputerActionProposal {
    pub action: String,
    pub target: HashMap<String, ProposalValue>,
    pub parameters: HashMap<String, ProposalValue>,
    pub provider_hint: Option<String>,
}

/// Scalar proposal values with finite bounds.
#[derive(Debug, Clone, PartialEq)]
pub enum ProposalValue {
    Text(String),
    Integer(i64),
    Number(f64),
    Flag(bool),
}

fn deny_forbidden(name: &str) -> Result<(), HostError> {
    if FORBIDDEN_PROPOSAL_FIELDS.contains(&name) {
        return Err(HostError::Invalid(
            "proposal authority smuggling denied".into(),
        ));
    }
    Ok(())
}

fn check_text(text: &str) -> Result<(), HostError> {
    if text.len() > MAX_TARGET_FIELD_BYTES {
        return Err(HostError::Invalid("proposal field oversized".into()));
    }
    if text
        .chars()
        .any(|ch| ch.is_control() && ch != '\n' && ch != '\t')
    {
        return Err(HostError::Invalid(
            "proposal control characters denied".into(),
        ));
    }
    Ok(())
}

fn check_number(value: f64) -> Result<(), HostError> {
    if !value.is_finite() {
        return Err(HostError::Invalid(
            "proposal non-finite number denied".into(),
        ));
    }
    if value.abs() > 9_007_199_254_740_992.0 {
        return Err(HostError::Invalid("proposal number overflow denied".into()));
    }
    Ok(())
}

/// Parse and validate a canonical proposal document. Unknown verbs,
/// unknown fields, forbidden fields at any level, oversized
/// documents, truncated input, and non-finite numbers fail closed.
pub fn parse_proposal(document: &str) -> Result<ComputerActionProposal, HostError> {
    if document.is_empty() || document.len() > MAX_PROPOSAL_BYTES {
        return Err(HostError::Invalid(
            "proposal document empty or oversized".into(),
        ));
    }
    let parsed: serde_json_like::Document = serde_json_like::parse(document)?;
    parsed.into_proposal()
}

/// Minimal JSON surface used by proposals. Implemented locally so the
/// adapter boundary has no donor or network dependency.
mod serde_json_like {
    use super::{check_number, check_text, deny_forbidden, ComputerActionProposal, ProposalValue};
    use super::{PROPOSAL_ENVELOPE_FIELDS, PROPOSAL_TARGET_FIELDS};
    use crate::error::HostError;
    use std::collections::HashMap;

    pub struct Document {
        pub action: String,
        pub target: HashMap<String, ProposalValue>,
        pub parameters: HashMap<String, ProposalValue>,
        pub provider_hint: Option<String>,
    }

    impl Document {
        pub fn into_proposal(self) -> Result<ComputerActionProposal, HostError> {
            if !super::PROPOSAL_ACTION_VERBS.contains(&self.action.as_str()) {
                return Err(HostError::Invalid("proposal unknown action denied".into()));
            }
            Ok(ComputerActionProposal {
                action: self.action,
                target: self.target,
                parameters: self.parameters,
                provider_hint: self.provider_hint,
            })
        }
    }

    enum Value {
        Text(String),
        Integer(i64),
        Number(f64),
        Flag(bool),
        Object(HashMap<String, Value>),
        Null,
    }

    struct Parser<'a> {
        bytes: &'a [u8],
        position: usize,
    }

    impl<'a> Parser<'a> {
        fn new(text: &'a str) -> Self {
            Self {
                bytes: text.as_bytes(),
                position: 0,
            }
        }

        fn fail(&self, what: &str) -> HostError {
            HostError::Invalid(format!("proposal malformed: {what}"))
        }

        fn peek(&self) -> Option<u8> {
            self.bytes.get(self.position).copied()
        }

        fn skip_gap(&mut self) {
            while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
                self.position += 1;
            }
        }

        fn expect(&mut self, byte: u8) -> Result<(), HostError> {
            self.skip_gap();
            if self.peek() == Some(byte) {
                self.position += 1;
                return Ok(());
            }
            Err(self.fail("unexpected token"))
        }

        fn parse_string(&mut self) -> Result<String, HostError> {
            self.expect(b'"')?;
            let mut out = String::new();
            loop {
                let byte = self.peek().ok_or_else(|| self.fail("truncated string"))?;
                self.position += 1;
                match byte {
                    b'"' => break,
                    b'\\' => {
                        let escaped = self.peek().ok_or_else(|| self.fail("truncated escape"))?;
                        self.position += 1;
                        match escaped {
                            b'"' | b'\\' | b'/' => out.push(escaped as char),
                            b'n' => out.push('\n'),
                            b't' => out.push('\t'),
                            _ => return Err(self.fail("unsupported escape")),
                        }
                    }
                    0x00..=0x1F => return Err(self.fail("control character")),
                    _ => out.push(byte as char),
                }
            }
            check_text(&out)?;
            Ok(out)
        }

        fn parse_number(&mut self) -> Result<Value, HostError> {
            let start = self.position;
            if self.peek() == Some(b'-') {
                self.position += 1;
            }
            let mut digits = 0;
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.position += 1;
                digits += 1;
            }
            let mut is_float = false;
            if self.peek() == Some(b'.') {
                is_float = true;
                self.position += 1;
                while matches!(self.peek(), Some(b'0'..=b'9')) {
                    self.position += 1;
                    digits += 1;
                }
            }
            if matches!(self.peek(), Some(b'e' | b'E')) {
                is_float = true;
                self.position += 1;
                if matches!(self.peek(), Some(b'+' | b'-')) {
                    self.position += 1;
                }
                while matches!(self.peek(), Some(b'0'..=b'9')) {
                    self.position += 1;
                    digits += 1;
                }
            }
            if digits == 0 || digits > 19 {
                return Err(self.fail("number malformed or overflowing"));
            }
            let text = std::str::from_utf8(&self.bytes[start..self.position])
                .map_err(|_| self.fail("number encoding"))?;
            if is_float {
                let value: f64 = text.parse().map_err(|_| self.fail("number malformed"))?;
                check_number(value)?;
                Ok(Value::Number(value))
            } else {
                let value: i64 = text.parse().map_err(|_| self.fail("integer overflow"))?;
                Ok(Value::Integer(value))
            }
        }

        fn parse_literal(&mut self) -> Result<Value, HostError> {
            for word in ["true", "false", "null"] {
                if self.bytes[self.position..].starts_with(word.as_bytes()) {
                    self.position += word.len();
                    return Ok(match word {
                        "true" => Value::Flag(true),
                        "false" => Value::Flag(false),
                        _ => Value::Null,
                    });
                }
            }
            Err(self.fail("unknown literal"))
        }

        fn parse_value(&mut self) -> Result<Value, HostError> {
            self.skip_gap();
            match self.peek() {
                Some(b'"') => Ok(Value::Text(self.parse_string()?)),
                Some(b'{') => {
                    self.position += 1;
                    let mut map = HashMap::new();
                    self.skip_gap();
                    if self.peek() == Some(b'}') {
                        self.position += 1;
                        return Ok(Value::Object(map));
                    }
                    loop {
                        self.skip_gap();
                        if self.peek() != Some(b'"') {
                            return Err(self.fail("object key must be a string"));
                        }
                        let key = self.parse_string()?;
                        self.expect(b':')?;
                        let value = self.parse_value()?;
                        if map.insert(key, value).is_some() {
                            return Err(self.fail("ambiguous duplicate key"));
                        }
                        self.skip_gap();
                        match self.peek() {
                            Some(b',') => {
                                self.position += 1;
                            }
                            Some(b'}') => {
                                self.position += 1;
                                break;
                            }
                            _ => return Err(self.fail("object separator")),
                        }
                    }
                    Ok(Value::Object(map))
                }
                Some(b'-' | b'0'..=b'9') => self.parse_number(),
                _ => self.parse_literal(),
            }
        }

        fn finish(&mut self) -> Result<(), HostError> {
            self.skip_gap();
            if self.position != self.bytes.len() {
                return Err(self.fail("trailing content"));
            }
            Ok(())
        }
    }

    fn scalar(value: Value, field: &str) -> Result<ProposalValue, HostError> {
        match value {
            Value::Text(text) => {
                check_text(&text)?;
                Ok(ProposalValue::Text(text))
            }
            Value::Integer(number) => Ok(ProposalValue::Integer(number)),
            Value::Number(number) => {
                check_number(number)?;
                Ok(ProposalValue::Number(number))
            }
            Value::Flag(flag) => Ok(ProposalValue::Flag(flag)),
            Value::Object(_) => Err(HostError::Invalid(format!(
                "proposal nested object denied: {field}"
            ))),
            Value::Null => Err(HostError::Invalid(format!("proposal null denied: {field}"))),
        }
    }

    pub fn parse(document: &str) -> Result<Document, HostError> {
        let mut parser = Parser::new(document);
        parser.skip_gap();
        if parser.peek() != Some(b'{') {
            return Err(HostError::Invalid(
                "proposal malformed: root must be an object".into(),
            ));
        }
        let root = match parser.parse_value() {
            Ok(Value::Object(map)) => map,
            Ok(_) => {
                return Err(HostError::Invalid(
                    "proposal malformed: root must be an object".into(),
                ))
            }
            Err(err) => return Err(err),
        };
        parser.finish()?;
        let mut action: Option<String> = None;
        let mut target: HashMap<String, ProposalValue> = HashMap::new();
        let mut parameters: HashMap<String, ProposalValue> = HashMap::new();
        let mut provider_hint: Option<String> = None;
        for (key, value) in root {
            deny_forbidden(&key)?;
            if !PROPOSAL_ENVELOPE_FIELDS.contains(&key.as_str()) {
                return Err(HostError::Invalid("proposal unknown envelope field".into()));
            }
            match (key.as_str(), value) {
                ("action", Value::Text(text)) => {
                    check_text(&text)?;
                    if action.is_some() {
                        return Err(HostError::Invalid(
                            "proposal malformed: duplicate action".into(),
                        ));
                    }
                    action = Some(text);
                }
                ("provider_hint", Value::Text(text)) => {
                    check_text(&text)?;
                    provider_hint = Some(text);
                }
                ("target", Value::Object(map)) => {
                    for (field, field_value) in map {
                        deny_forbidden(&field)?;
                        if !PROPOSAL_TARGET_FIELDS.contains(&field.as_str()) {
                            return Err(HostError::Invalid("proposal unknown target field".into()));
                        }
                        if target
                            .insert(field.clone(), scalar(field_value, &field)?)
                            .is_some()
                        {
                            return Err(HostError::Invalid(
                                "proposal malformed: duplicate target".into(),
                            ));
                        }
                    }
                }
                ("parameters", Value::Object(map)) => {
                    for (field, field_value) in map {
                        deny_forbidden(&field)?;
                        check_text(&field)?;
                        if parameters
                            .insert(field.clone(), scalar(field_value, &field)?)
                            .is_some()
                        {
                            return Err(HostError::Invalid(
                                "proposal malformed: duplicate parameter".into(),
                            ));
                        }
                    }
                }
                _ => return Err(HostError::Invalid("proposal envelope shape denied".into())),
            }
        }
        let action = action.ok_or_else(|| HostError::Invalid("proposal action missing".into()))?;
        check_text(&action)?;
        Ok(Document {
            action,
            target,
            parameters,
            provider_hint,
        })
    }
}

/// Donor output record shape accepted by the UI-TARS adapter. Syntax
/// only: an action name with finite numeric arguments in donor
/// 0-1000 space and optional text. No donor code is imported and no
/// authority travels with this record.
#[derive(Debug, Clone)]
pub struct UitarsRecord {
    pub action: String,
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub dx: Option<f64>,
    pub dy: Option<f64>,
    pub text: Option<String>,
}

/// Normalize a UI-TARS output record into the canonical IR. Unknown
/// actions, non-finite or out-of-range numbers, oversized text, and
/// any authority-bearing smuggling fail closed. The result is an
/// untrusted proposal, never executable authority.
pub fn adapt_uitars(record: &UitarsRecord) -> Result<ComputerActionProposal, HostError> {
    let verb = match record.action.to_ascii_lowercase().as_str() {
        "click" => "coordinate_click",
        "double_click" | "doubleclick" => "coordinate_double_click",
        "right_click" | "rightclick" => "coordinate_right_click",
        "scroll" => "coordinate_scroll",
        "type" => "coordinate_type",
        _ => return Err(HostError::Invalid("uitars unknown action denied".into())),
    };
    let mut parameters: HashMap<String, ProposalValue> = HashMap::new();
    let mut coordinate = |name: &str, value: Option<f64>| -> Result<(), HostError> {
        if let Some(number) = value {
            if !number.is_finite() {
                return Err(HostError::Invalid(
                    "uitars non-finite coordinate denied".into(),
                ));
            }
            if !(0.0..=1000.0).contains(&number) {
                return Err(HostError::Invalid("uitars coordinate out of range".into()));
            }
            parameters.insert(name.to_owned(), ProposalValue::Number(number));
        }
        Ok(())
    };
    match verb {
        "coordinate_click" | "coordinate_double_click" | "coordinate_right_click" => {
            let (x, y) = match (record.x, record.y) {
                (Some(x), Some(y)) => (x, y),
                _ => {
                    return Err(HostError::Invalid(
                        "uitars click coordinates missing".into(),
                    ))
                }
            };
            coordinate("x", Some(x))?;
            coordinate("y", Some(y))?;
        }
        "coordinate_scroll" => {
            let (dx, dy) = match (record.dx, record.dy) {
                (Some(dx), Some(dy)) => (dx, dy),
                _ => return Err(HostError::Invalid("uitars scroll delta missing".into())),
            };
            coordinate("dx", Some(dx))?;
            coordinate("dy", Some(dy))?;
        }
        "coordinate_type" => {
            let text = record
                .text
                .as_deref()
                .ok_or_else(|| HostError::Invalid("uitars text missing".into()))?;
            if text.is_empty() || text.len() > crate::coordinates::MAX_INPUT_TEXT_CHARS {
                return Err(HostError::Invalid("uitars text out of bounds".into()));
            }
            if text
                .chars()
                .any(|ch| ch.is_control() && ch != '\n' && ch != '\t')
            {
                return Err(HostError::Invalid("uitars text control denied".into()));
            }
            parameters.insert("text".to_owned(), ProposalValue::Text(text.to_owned()));
        }
        _ => return Err(HostError::Invalid("uitars unknown action denied".into())),
    }
    if record.text.is_some() && verb != "coordinate_type" {
        return Err(HostError::Invalid("uitars unexpected text denied".into()));
    }
    Ok(ComputerActionProposal {
        action: verb.to_owned(),
        target: HashMap::new(),
        parameters,
        provider_hint: Some("ui-tars".into()),
    })
}

/// Denied: adapters never invoke backends directly.
pub fn invoke_backend_directly(_proposal: &ComputerActionProposal) -> Result<(), HostError> {
    Err(HostError::Invalid(
        "direct backend invocation denied".into(),
    ))
}

/// Denied: proposals never mint target identities.
pub fn mint_target_identity() -> Result<(), HostError> {
    Err(HostError::Invalid("proposal target minting denied".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn envelope(action: &str) -> String {
        format!(r#"{{"action":{action:?},"target":{{}},"parameters":{{}}}}"#)
    }

    #[test]
    fn all_sixteen_verbs_parse_and_unknown_verbs_fail() {
        for verb in PROPOSAL_ACTION_VERBS {
            assert!(parse_proposal(&envelope(verb)).is_ok(), "{verb}");
        }
        assert_eq!(PROPOSAL_ACTION_VERBS.len(), 16);
        assert!(parse_proposal(&envelope("browser_evaluate")).is_err());
        assert!(parse_proposal(&envelope("shell.run")).is_err());
    }

    #[test]
    fn malformed_oversized_truncated_and_ambiguous_fail_closed() {
        assert!(parse_proposal("").is_err());
        assert!(parse_proposal("{").is_err());
        assert!(parse_proposal(r#"{"action":"coordinate_click""#).is_err());
        assert!(parse_proposal(&" ".repeat(MAX_PROPOSAL_BYTES + 1)).is_err());
        assert!(parse_proposal(r#"{"action":"coordinate_click","action":"coordinate_click","target":{},"parameters":{}}"#).is_err());
        assert!(parse_proposal(
            r#"{"action":"coordinate_click","target":{},"parameters":{},"extra":1}"#
        )
        .is_err());
        assert!(parse_proposal(r#"{"target":{},"parameters":{}}"#).is_err());
        assert!(parse_proposal(
            r#"{"action":"coordinate_click","target":{"unknown_field":"x"},"parameters":{}}"#
        )
        .is_err());
    }

    #[test]
    fn authority_smuggling_fields_fail_everywhere() {
        for forbidden in FORBIDDEN_PROPOSAL_FIELDS {
            assert!(
                parse_proposal(&format!(
                    r#"{{"action":"coordinate_click","target":{{}},"parameters":{{}}," {forbidden} ":1}}"#
                ))
                .is_err(),
                "{forbidden}"
            );
            assert!(
                parse_proposal(&format!(
                    r#"{{"action":"coordinate_click","target":{{"{forbidden}":"x"}},"parameters":{{}}}}"#
                ))
                .is_err(),
                "{forbidden}"
            );
            assert!(
                parse_proposal(&format!(
                    r#"{{"action":"coordinate_click","target":{{}},"parameters":{{"{forbidden}":"x"}}}}"#
                ))
                .is_err(),
                "{forbidden}"
            );
        }
        assert_eq!(FORBIDDEN_PROPOSAL_FIELDS.len(), 10);
    }

    #[test]
    fn non_finite_overflow_and_forged_values_fail_closed() {
        assert!(parse_proposal(
            r#"{"action":"coordinate_click","target":{},"parameters":{"x":1e400}}"#
        )
        .is_err());
        assert!(parse_proposal(
            r#"{"action":"coordinate_click","target":{},"parameters":{"x":99999999999999999999}}"#
        )
        .is_err());
        assert!(parse_proposal(
            r#"{"action":"coordinate_click","target":{"page_id":"../escape"},"parameters":{}}"#
        )
        .is_ok());
        let mut forged = parse_proposal(&envelope("coordinate_click")).unwrap();
        forged.target.insert(
            "page_id".to_owned(),
            ProposalValue::Text("../escape".to_owned()),
        );
        assert!(crate::observation::validate_identity_field("../escape").is_err());
        assert!(mint_target_identity().is_err());
        assert!(invoke_backend_directly(&forged).is_err());
    }

    #[test]
    fn uitars_adapter_normalizes_syntax_only() {
        let record = UitarsRecord {
            action: "click".into(),
            x: Some(500.0),
            y: Some(250.0),
            dx: None,
            dy: None,
            text: None,
        };
        let proposal = adapt_uitars(&record).unwrap();
        assert_eq!(proposal.action, "coordinate_click");
        assert_eq!(proposal.provider_hint.as_deref(), Some("ui-tars"));
        assert!(proposal.target.is_empty());
        let unknown = UitarsRecord {
            action: "shell".into(),
            x: None,
            y: None,
            dx: None,
            dy: None,
            text: None,
        };
        assert!(adapt_uitars(&unknown).is_err());
        let nan = UitarsRecord {
            action: "click".into(),
            x: Some(f64::NAN),
            y: Some(1.0),
            dx: None,
            dy: None,
            text: None,
        };
        assert!(adapt_uitars(&nan).is_err());
        let infinite = UitarsRecord {
            action: "scroll".into(),
            x: None,
            y: None,
            dx: Some(f64::INFINITY),
            dy: Some(0.0),
            text: None,
        };
        assert!(adapt_uitars(&infinite).is_err());
        let overflow = UitarsRecord {
            action: "click".into(),
            x: Some(1001.0),
            y: Some(1.0),
            dx: None,
            dy: None,
            text: None,
        };
        assert!(adapt_uitars(&overflow).is_err());
    }

    #[test]
    fn adapter_and_json_agree_differentially() {
        let record = UitarsRecord {
            action: "double_click".into(),
            x: Some(10.0),
            y: Some(20.0),
            dx: None,
            dy: None,
            text: None,
        };
        let adapted = adapt_uitars(&record).unwrap();
        let parsed = parse_proposal(
            r#"{"action":"coordinate_double_click","target":{},"parameters":{"x":10.0,"y":20.0}}"#,
        )
        .unwrap();
        assert_eq!(adapted.action, parsed.action);
        assert_eq!(adapted.parameters, parsed.parameters);
    }
}
