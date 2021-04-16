use std::{
    error::Error,
    fmt,
};

use super::filter::FilterOp;


#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FunctionParseError {
    InvalidArguments(String),
    MissingClosingParenthesis,
    NoVariableAssignment(String),
    InvalidOperator(FilterOp),
    UnknownFunction(String),
}

impl fmt::Display for FunctionParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FunctionParseError::InvalidArguments(s) =>
                write!(f, "Invalid argument: {}", s),
            FunctionParseError::MissingClosingParenthesis =>
                write!(f, "Missing closing parenthesis"),
            FunctionParseError::NoVariableAssignment(s) =>
                write!(f, "Variable assignment from function expected; received: {}", s),
            FunctionParseError::InvalidOperator(op) =>
                write!(f, "Invalid operator: {}", op),
            FunctionParseError::UnknownFunction(s) =>
                write!(f, "Unknown function: {}", s),
        }
    }
}

impl Error for FunctionParseError {}

#[derive(Debug)]
pub enum ConditionConversionError {
    MismatchedParenthesis,
    Invalid(String),
    UnknownOperator(String),
    MissingOperator,
    MissingField,
    InvalidFieldName(String),
    BadComparison(String),
    UnquotedString(String),
    Function(FunctionParseError),
}

impl Error for ConditionConversionError {}

impl From<FunctionParseError> for ConditionConversionError {
    fn from(e: FunctionParseError) -> Self {
        Self::Function(e)
    }
}

impl fmt::Display for ConditionConversionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MismatchedParenthesis =>
                write!(f, "Missing or mismatched parenthesis in expression"),
            Self::Invalid(s) => write!(f, "Invalid condition string: {}", s),
            Self::UnknownOperator(s) => write!(f, "Unknown operator: {}", s),
            Self::MissingOperator =>
                write!(f, "Missing or invalid operator clause"),
            Self::MissingField => write!(f, "Expected a field name"),
            Self::InvalidFieldName(s) => write!(f, "Invalid field name: {}", s),
            Self::BadComparison(s) => write!(f, "{}", s),
            Self::UnquotedString(s) =>
                write!(f, "String literal is not quoted: {}", s),
            Self::Function(e) => write!(f, "{}", e),
        }
    }
}

#[derive(Debug)]
pub enum QueryConversionError {
    MissingWhere(String),
    Condition(ConditionConversionError),
}

impl Error for QueryConversionError {}

impl From<ConditionConversionError> for QueryConversionError {
    fn from(e: ConditionConversionError) -> Self {
        Self::Condition(e)
    }
}

impl fmt::Display for QueryConversionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingWhere(s) =>
                write!(f, "Expected WHERE clause; found: {}", s),
            Self::Condition(c) => write!(f, "{}", c),
        }
    }
}
