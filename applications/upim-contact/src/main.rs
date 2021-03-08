#![feature(assoc_char_funcs)]
#![feature(iter_advance_by)]
#![feature(option_result_contains)]
#![feature(pattern)]
#![feature(str_split_once)]
#![allow(dead_code)] // TODO: Only for early development

mod args;
mod contact;
mod either;
mod filter;

use std::{
    path::PathBuf,
    str::FromStr as _,
    env,
    fmt,
};

use anyhow::anyhow;

use upim_core::{
    config::{Config, find_application_configuration},
    error::FileError,
};

use args::{Command, Options};
use filter::Query;


fn main() -> anyhow::Result<()> {
    let opts = Options::new(env::args())?;

    let conf = read_config(opts.conf_path)
        .map_err(|errs| {
            for e in errs {
                eprintln!("Error: {}", e);
            }
            return anyhow!("Failed to read configuration file.");
        })?;

    let search = match opts.cmd_or_alias {
        Command::Search => opts.filter,
        Command::Alias(ref name) => {
            match conf.get("Aliases", name) {
                Some(alias) => {
                    let alias = Query::from_str(alias)?;

                    opts.filter.map(|f1| { alias.merge_with(f1) })
                },
                None => return Err(anyhow!("Unknown alias: {}", name)),
            }
        },
        Command::New(_name) => {
            todo!();
            None
        },
        Command::Edit(_name) => {
            // TODO: match name
            todo!();
            None
        },
    };

    if let Some(_search) = search {
        // TODO: Perform the search.
    };

    Ok(())
}

/// Get the path to the first upim-contact.conf file found (upim-contact.ini on
/// Windows).
fn find_default_configuration() -> Option<PathBuf> {
    find_application_configuration("upim-contact")
}

fn read_config(path: Option<PathBuf>)
-> std::result::Result<Config, Vec<ConfigurationError>> {
    let path = path.or_else(find_default_configuration);

    let mut conf = Config::default()
        .set_default("field_separator", " | ");

    let mut errors = vec![];

    if let Some(path) = path {
        let config = Config::read_from_file(&path)
            .map_err(|v| v.iter()
                .map(|e| ConfigurationError::Config(e.clone()))
                    .collect::<Vec<ConfigurationError>>());

        match config {
            Ok(c) => conf = conf.merge_with(c),
            Err(errs) => errors = errs,
        };
    } else {
        return Err(vec![
            ConfigurationError::Environment(
                "No configuration file found".into()
            )
        ]);
    };

    if conf.get_default("default_collection").is_none() {
        errors.push(
            ConfigurationError::MissingOption("default_collection".into())
        );
    }

    match validate_field_separator(conf.get_default("field_separator").unwrap())
    {
        Ok(ref v) => conf = conf.set_default("field_separator", v),
        Err(e) => errors.push(e),
    }

    /* TODO
    if let Err(mut errs) = validate_aliases(conf.variables_in_group("Aliases"))
    {
        errors.append(&mut errs);
    }
    */

    if errors.is_empty() {
        Ok(conf)
    } else {
        Err(errors)
    }
}

/// Errors that can occur while reading information from our exteral
/// environment.
#[derive(Debug, Clone)]
pub enum ConfigurationError {
    Config(FileError),
    InvalidValue { data: String, rules: String },
    MissingOption(String),
    Environment(String),
}

impl fmt::Display for ConfigurationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ConfigurationError::Config(ref e) => e.fmt(f),
            ConfigurationError::InvalidValue { ref data, ref rules } =>
                write!(f, "Invalid value: {}. {}", data, rules),
            ConfigurationError::MissingOption(ref s) =>
                write!(f, "Missing option: {}", s),
            ConfigurationError::Environment(ref s) => write!(f, "{}", s),
        }
    }
}

impl std::error::Error for ConfigurationError {}

impl From<FileError> for ConfigurationError {
    fn from(err: FileError) -> ConfigurationError {
        ConfigurationError::Config(err)
    }
}

/// Validate the field separator and interpret special values.
///
/// - Replaces "{SPACE}" or "{TAB}" with a space or tab, respectively.
/// - Replaces a Unicode character code in the form \uXXXX with the character
///   code itself.
pub fn validate_field_separator(val: &str)
-> std::result::Result<String, ConfigurationError> {
    use crate::filter::is_quoted;

    if val.len() > 1 && !is_quoted(val) {
        return Err(ConfigurationError::InvalidValue {
            data: val.into(),
            rules: "field_separator strings must be quoted".into(),
        })
    }

    let val = match is_quoted(val) {
        true => &val[1..val.len()-1],
        false => val
    };

    let val = val.replace("{SPACE}", " ")
        .replace("{TAB}", "\t");

    let val = match unescape_unicode(&val) {
        Ok(v) => v,
        Err(e) => {
            return Err(ConfigurationError::InvalidValue {
                data: e.0,
                rules: "Invalid Unicode escape sequence".into(),
            });
        }
    };

    Ok(val)
}

pub fn validate_aliases<'a, I>(aliases: I)
-> std::result::Result::<(), Vec<ConfigurationError>>
    where I: Iterator<Item = &'a String>,
{
    let mut errors = vec![];

    // TODO: Take alias name as param for error messages.
    // TODO: Real errors for Query::from_str so we can do better here.

    for alias in aliases {
        let parts = alias.split("--");
        for part in parts {
            // TODO: Validate all possible options
            if let Some(filter) = part.strip_prefix("filter") {
                if let Err(e) = Query::from_str(&filter.trim_start()) {
                    errors.push(ConfigurationError::InvalidValue {
                        data: format!("{}", e),
                        rules: "Invalid filter".into(),
                    });
                }
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[derive(Debug)]
pub struct UnescapeError(String);

/// Unescape Unicode escape sequences within the given string.
///
/// Scans the full string and processes all escape sequences. Always returns a
/// new string, even if no substitutions were made.
pub fn unescape_unicode(s: &str)
-> std::result::Result<String, UnescapeError> {
    let mut out_str = String::default();

    // TODO: Check the optimizer on this.
    let is_not_hexadecimal = |c: char| {
        ! (char::is_numeric(c)
            || ('a'..='f').contains(&c)
            || ('A'..='F').contains(&c))
    };

    let mut chars = s.chars();
    loop {
        match chars.next() {
            Some('\\') => {
                let ch_str = chars.as_str();

                if ! ch_str.starts_with('u') {
                    return Err(UnescapeError(ch_str.to_owned()));
                }

                let code = ch_str[1..]
                    .split(is_not_hexadecimal)
                    .next()
                    .ok_or_else(|| UnescapeError(ch_str.to_owned()))?;

                if code.len() != 4 {
                    return Err(UnescapeError(ch_str.to_owned()));
                } else {
                    chars.advance_by(5)
                        .map_err(|_| UnescapeError(ch_str.to_owned()))?;
                }

                let u = u32::from_str_radix(&code, 16)
                    .map_err(|_| UnescapeError(ch_str.to_owned()))?;

                match char::from_u32(u) {
                    Some(c) => out_str.push(c),
                    None => return Err(UnescapeError(ch_str.to_owned()))
                }
            },
            Some(c) => out_str.push(c),
            None => break,
        }
    }

    Ok(out_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_default_aliases() {
        let aliases = vec![
            "--filter 'Name,Phone,Employer:Name' WHERE Name = '$0' --limit 1",
            "--filter 'Name,Phone,Employer:Name' WHERE Name = '$0'",
        ];
        let aliases = aliases.iter()
            .map(|s| s.to_string())
            .collect::<Vec<String>>();

        assert!(validate_aliases(aliases.iter()).is_ok())
    }

    #[test]
    fn error_when_validating_bad_filter() {
        let aliases = vec![
            "--filter Name,Phone,Employer:Name' WHERE Name = '$0' --limit 1",
            "--filter Name,Phone,Employer:Name' WHERE Name > '$0'",
            "--filter 'Name,Phone,Employer:Name' WHERE",
        ];

        let aliases = aliases.iter()
            .map(|s| vec![s.to_string()])
            .collect::<Vec<Vec<String>>>();

        for alias in aliases {
            assert!(validate_aliases(alias.iter()).is_err())
        }
    }

    #[test]
    fn unescape_unicode_valid_input() {
        let text = r"This is a \u2764!";
        let unescaped = "This is a ❤!";

        assert!(text != unescaped);
        assert_eq!(unescape_unicode(text).unwrap(), unescaped);
    }

    #[test]
    fn unescape_unicode_not_hex_is_err() {
        assert!(unescape_unicode(r"\ughij").is_err());
    }

    #[test]
    fn unescape_unicode_not_four_chars_is_err() {
        assert!(unescape_unicode(r"\u9").is_err());
        assert!(unescape_unicode(r"\u99").is_err());
        assert!(unescape_unicode(r"\u999").is_err());
        assert!(unescape_unicode(r"\u99999").is_err());
    }

    #[test]
    fn unescape_unicode_invalid_codepoint_is_err() {
        assert!(unescape_unicode(r"\ud8f3").is_err());
    }

    #[test]
    fn validate_string_field_separator() {
        assert_eq!(validate_field_separator("' :: '").unwrap(), " :: ");
    }

    #[test]
    fn validate_string_char_separator() {
        assert_eq!(validate_field_separator("#").unwrap(), "#");
    }

    #[test]
    fn validate_string_whitespace_tag_in_separator() {
        assert_eq!(validate_field_separator("'{SPACE}{TAB}'").unwrap(), " \t");
    }

    #[test]
    fn validate_string_unicode_code_points_in_separator() {
        assert_eq!(
            validate_field_separator(r"'\u2713\u27fa\u27F4'").unwrap(),
            "✓⟺⟴"
        );
    }
}
