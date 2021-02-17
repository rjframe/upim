//! Contact filter
//!
//! Contact filters are almost SQL "SELECT FROM" queries.
//!
//! (TODO: Finish filter description, add EBNF)

use std::str::FromStr;

use anyhow::anyhow;


/// Supported operators on filters.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum FilterOp {
    EqualTo,
    LessThan,
    LessEq,
    GreaterThan,
    GreaterEq,
    Not,
}

impl Default for FilterOp {
    fn default() -> Self { Self::EqualTo }
}

impl FromStr for FilterOp {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self> {
        match s {
            "="   => Ok(Self::EqualTo),
            "<"   => Ok(Self::LessThan),
            "<="  => Ok(Self::LessEq),
            ">"   => Ok(Self::GreaterThan),
            ">="  => Ok(Self::GreaterEq),
            "NOT" => Ok(Self::Not),
            _ => Err(anyhow!("Invalid string for Filter operator: {}", s))
        }
    }
}

/// Supported functions in queries.
#[derive(Clone, Debug, Eq, PartialEq)]
enum Function {
    /// Look up the value of the included field as a subcontact for the query.
    Ref(String),
    /// Split the value of the included field by the specified character. Treat
    /// each split as an individual value.
    Split(String, char),
    /// Match the given field's value against the provided regular expression.
    Regex(String, String),
}

impl FromStr for Function {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self> {
        panic!();
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Condition {
    All, // Unfiltered.
    // Field, op, value
    Filter(String, FilterOp, String),
    // Identifier = Function; e.g., "s = REF(Spouse)"
    Function(String, Function),
    // Logical and with the contained conditions.
    And(Box<(Condition, Condition)>),
    // Logical or with the contained conditions.
    Or(Box<(Condition, Condition)>),
}

impl Default for Condition {
    fn default() -> Self { Self::All }
}

impl FromStr for Condition {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self> {
        let mut s = s.trim_start();

        let (len, field) = read_field(s)?;
        s = &s[len..s.len()].trim_start();

        let (len, op) = read_op(s)?;
        s = &s[len..s.len()].trim_start();

        // TODO: If the remaining string is a function...

        Ok(Self::Filter(field, op, s.into()))

        // TODO: If the remaining string is AND/OR...
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Filter {
    /// The fields to return.
    select: Vec<String>,
    /// The filter condition.
    condition: Condition,
}

impl FromStr for Filter {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self> {
        let mut s = s;
        let mut f = Self::default();

        let (idx, select) = read_fields(&s)?;
        assert!(idx <= s.len());
        f.select = select;

        // A field-only filter (no WHERE clause) is valid.
        if idx == s.trim_end().len() {
            f.condition = Condition::All;
            return Ok(f);
        }

        s = s[idx..s.len()].trim_start();
        if s.is_empty() { return Ok(f); }

        if s.len() < 6 || s[0..5].to_ascii_uppercase() != "WHERE" {
            return Err(anyhow!("Expected WHERE clause: found: {}", s));
        } else {
            s = s[5..s.len()].trim_start();
        }

        f.condition = Condition::from_str(&s)?;

        Ok(f)
    }
}

/// Read a single field from the input string.
///
/// # Returns
///
/// Returns the number of characers read and the field name.
fn read_field(s: &str) -> anyhow::Result<(usize, String)> {
    let (start_idx, end_idx) = {
        let (start_idx, end_char) = match s.chars().next() {
            Some('\'') => (1, '\''),
            Some('"') => (1, '"'),
            Some(c) => (0, ' '),
            None => return Err(anyhow!("Expected a field name"))
        };

        let end_idx = s[1..s.len()]
            .find(end_char)
            .map(|i| i + 1); // Take us to the char past the end.

        (start_idx, end_idx)
    };

    match end_idx {
        Some(i) => {
            Ok((
                i + start_idx, // Re-add the skipped quote if necessary.
                s[start_idx..i].into()
            ))
        },
        None => Err(anyhow!("Invalid field string: {}", s))
    }
}

/// Read a list of fields from the input string.
///
/// # Returns
///
/// Returns the number of characters read and the list of fields.
fn read_fields(s: &str) -> anyhow::Result<(usize, Vec<String>)> {
    // TODO: Validate the field names here and with read_field. Need list of
    // requirements.
    // - no " WHERE "
    // - no quote marks
    // TODO: Refactor. This function is ugly.

    let (start_idx, end_idx) = {
        let (start_idx, end_char) = match s.chars().next() {
            Some('\'') => (1, '\''),
            Some('"') => (1, '"'),
            Some(c) => (0, ' '),
            None => return Ok((0, vec![])),
        };

        // If the end_char is a space and we don't have one later, the input is
        // a list of fields with no WHERE clause. This is valid -- We'll print
        // the fields for all contacts.
        let end_idx = s[1..s.len()].find(end_char)
            .map(|i| i + 1) // Take us to the char past the end.
            .or_else(|| if end_char == ' ' { Some(s.len()) } else { None })
            ;

        (start_idx, end_idx)
    };

    match end_idx {
        Some(i) => {
            Ok((
                i + start_idx, // Re-add the skipped quote if necessary.
                s[start_idx..i]
                    .split(',')
                    .map(|s| s.to_string())
                    .collect()
            ))
        },
        None => Err(anyhow!("Expected closing quote in field list"))
    }
}

/// Read a filter operator from the input string.
///
/// # Returns
///
/// Returns the number of characters read and the [FilterOp | operator].
fn read_op(s: &str) -> anyhow::Result<(usize, FilterOp)> {
    if let Some((op, _)) = s.split_once(' ') {
        let operator = FilterOp::from_str(op)?;
        Ok((op.len(), operator))
    } else {
        Err(anyhow!("Invalid operator clause: {}", s))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_single_field_no_quotes() {
        let text = "Field more text";

        let (len, fields) = read_fields(text).unwrap();
        assert_eq!(len, 5);
        assert_eq!(fields[0], "Field");
    }

    #[test]
    fn read_single_field_with_quotes() {
        let text = "'Field' more text";

        let (len, fields) = read_fields(text).unwrap();
        assert_eq!(len, 7);
        assert_eq!(fields[0], "Field");
    }

    #[test]
    fn read_multiple_fields_no_quotes() {
        let text = "AField,BField more text";

        let (len, fields) = read_fields(text).unwrap();
        assert_eq!(len, 13);
        assert_eq!(fields[0], "AField");
        assert_eq!(fields[1], "BField");
    }

    #[test]
    fn read_multiple_fields_with_quotes() {
        let text = "'A Field,B Field' more text";

        let (len, fields) = read_fields(text).unwrap();
        assert_eq!(len, 17);
        assert_eq!(fields[0], "A Field");
        assert_eq!(fields[1], "B Field");
    }

    #[test]
    fn read_operators() {
        let tests = vec![
            ("=", FilterOp::EqualTo),
            ("<", FilterOp::LessThan),
            ("<=", FilterOp::LessEq),
            (">", FilterOp::GreaterThan),
            (">=", FilterOp::GreaterEq),
            ("NOT", FilterOp::Not),
        ];

        for (s, op) in tests.iter() {
            assert_eq!(FilterOp::from_str(s).unwrap(), *op);
        }

        assert!(FilterOp::from_str("asdf").is_err());
    }

    #[test]
    fn parse_filter_all_contacts() {
        let text = "Name";

        let filter = Filter::from_str(text).unwrap();
        assert_eq!(filter,
            Filter {
                select: vec!["Name".into()],
                condition: Condition::All,
            });
    }

    #[test]
    fn parse_filter_all_quoted_contacts() {
        let text = "'Name'";

        let filter = Filter::from_str(text).unwrap();
        assert_eq!(filter,
            Filter {
                select: vec!["Name".into()],
                condition: Condition::All,
            });
    }

    #[test]
    fn parse_condition_by_field_value() {
        let text = "Name = 'Somebody'";

        let cond = Condition::from_str(text).unwrap();
        assert_eq!(cond,
            Condition::Filter(
                "Name".into(),
                FilterOp::EqualTo,
                "'Somebody'".into()
            )
        );
    }

    #[test]
    fn parse_condition_by_function() {
        let text = "v =  REF(SomeField)";

        let cond = Condition::from_str(text).unwrap();
        assert_eq!(cond,
            Condition::Function("v".into(), Function::Ref("SomeField".into()))
        );
    }
    // TODO: Test all supported functions.

    #[test]
    fn parse_filter_and_filter() {
        let text = "'s.Name,s.Phone' WHERE Name = 'Person' AND s = REF(Spouse)";

        let cond = Condition::from_str(text).unwrap();
        assert_eq!(cond,
            Condition::And(Box::new((
                Condition::Filter(
                    "Name".into(),
                    FilterOp::EqualTo,
                    "'Person'".into()
                ),
                Condition::Function("s".into(), Function::Ref("Spouse".into()))
            )))
        );
    }

    #[test]
    fn parse_filter_or_filter() {
        panic!();
    }

    #[test]
    fn parse_filter_by_field_value() {
        let text = "'Name' WHERE Name = 'Somebody'";

        let filter = Filter::from_str(text).unwrap();
        assert_eq!(filter,
            Filter {
                select: vec!["Name".into()],
                condition: Condition::Filter(
                    "Name".into(),
                    FilterOp::EqualTo,
                    "'Somebody'".into()
                ),
            });
    }
}
