//! Centralized configuration management for uPIM applications.
//!
//! uPIM uses a simple INI file format:
//!
//! - groups are created by naming them within brackets.
//! - an equal sign ('=') is used for variable assignment.
//! - variables can contain a '=' character.
//! - leading and trailing whitespace is ignored.
//! - whitespace surrounding group names, variables, and values are removed.
//! - whitespace within group names, variable names, and values is allowed.
//! - a semicolon (';') at the beginning of a line denotes a comment.
//! - if a variable is set multiple times in a file, the last one read is kept.
//!
//! Multiple INI files can be merged into a single [Config]; variables read in a
//! later file replace any set in prior configuration files.
//!
//! The configuration values can be modified via the [Config::set] method; `set`
//! also provides a convenient API for setting default values prior to reading a
//! configuration file.
//!
//! Writing to INI files is not supported.

use std::{
    collections::HashMap,
    ops::Index,
};

use super::error::FileError;


// TODO: Write to file.

/// Read the standard uPIM configuration.
///
/// Configurations are read from the following paths, in the following order:
///
/// On UNIX-like operating systems:
///
/// 1. /etc/upim/upim.conf XOR /etc/upim.conf
/// 2. $HOME/.config/upim/upim.conf XOR $HOME/.config/upim.conf
///    XOR $HOME/.upim.conf
/// 3. <current working directory>/.upim.conf
///
/// On Windows:
///
/// 1. %COMMONPROGRAMFILES%\uPIM\conf.ini
/// 2. %APPDATA\uPIM\conf.ini
/// 3. <current working directory>/upim.ini
///
/// Values set in later files override the earlier values, so the priority is in
/// the reverse order of the list above.
pub fn read_upim_configuration() -> Config {
    panic!();
}

/// The key used to look up a configuration value.
///
/// The key is a group/variable pair. The default group is "DEFAULT".
// This is not efficient for a large number of keys.
type Key = (String, String);

/// The Configuration object.
#[derive(Debug, Default)]
pub struct Config {
    values: HashMap<Key, String>,
}

impl Config {
    /// Read a [Config] from the INI file at the path specified.
    pub fn read_from_file(path: &str) -> Result<Self, FileError> {
        use std::{
            fs::File,
            io::{prelude::*, BufReader},
        };

        let f = File::open(path)?;
        let mut reader = BufReader::new(f);
        let mut line = String::new();

        let mut map = HashMap::new();
        let mut group = String::from("DEFAULT");

        // TODO: Track line numbers for error messages.
        while reader.read_line(&mut line)? > 0 {
            line = line.trim().into();
            if line.is_empty() { continue; }

            if line.starts_with(';') {
                line.clear();
                continue;
            } else if line.starts_with('[') {
                if line.ends_with(']') {
                    group = line[1..line.len()-1].trim().into();
                } else {
                    return Err(FileError::Parse {
                        msg: "Missing closing bracket for group name".into(),
                        data: line
                    });
                }
            } else if let Some((var, val)) = line.split_once('=') {
                    map.insert(
                        (group.clone(), var.trim_end().to_string()),
                        val.trim_start().to_string()
                    );
            } else {
                return Err(FileError::Parse {
                    msg: "Expected a variable assignment".into(),
                    data: line
                });
            }
            line.clear();
        }

        Ok(Self { values: map })
    }

    /// Merge two [Config]s, consuming both of the originals.
    ///
    /// Any duplicate variables will contain the values in `other`.
    pub fn merge_with(mut self, other: Self) -> Self {
        for (k, v) in other.values {
            self.values.insert(k, v);
        }
        self
    }

    /// Add the specified value to the configuration.
    ///
    /// `set` can be used to create default settings by setting values prior to
    /// reading a configuration:
    ///
    /// ```
    /// use upim_core::Config;
    ///
    /// // If test.ini includes `var1 = val1` but does not have `some-var`:
    /// let conf = Config::default()
    ///     .set("DEFAULT", "var1", "default value")
    ///     .set("DEFAULT", "some-var", "my-value")
    ///     .merge_with(Config::read_from_file("test/test.ini").unwrap());
    ///
    /// assert_eq!(conf["var1"], "val1");
    /// assert_eq!(conf["some-var"], "my-value");
    /// ```
    pub fn set(mut self, group: &str, var: &str, val: &str) -> Self {
        self.values.insert(
            (group.into(), var.into()),
            val.into()
        );
        self
    }

    /// Add the specified value to the configuration within the DEFAULT group.
    ///
    /// See [Config::set] for more information.
    pub fn set_default(self, var: &str, val: &str) -> Self {
        self.set("DEFAULT", var, val)
    }

    // TODO: Vec<&str>? Iterator?
    /// Get the list of groups in the configuration file.
    pub fn groups(&self) -> Vec<String> {
        self.values.keys().map(|k| k.0.clone()).collect()
    }

    // TODO: Vec<&str>? Iterator?
    /// Get the list of variables set in the specified group.
    pub fn variables_in_group(&self, group: &str) -> Vec<String> {
        self.values.keys()
            .filter_map(|k| {
                if k.0 == group { Some(k.1.clone()) } else { None }
            })
            .collect()
    }

    /// Retrieve the value of the specified variable within the DEFAULT group,
    /// or `None` if it is not set.
    pub fn get(&self, variable: &str) -> Option<&String> {
        self.values.get(&("DEFAULT".into(), variable.into()))
    }

    /// Retrieve the value of the specified variable within the specified group,
    /// or `None` if it is not set.
    pub fn get_group(&self, group: &str, variable: &str) -> Option<&String> {
        self.values.get(&(group.into(), variable.into()))
    }
}

impl Index<&str> for Config {
    type Output = String;

    fn index(&self, variable: &str) -> &Self::Output {
        self.index(("DEFAULT", variable))
    }
}

impl Index<(&str, &str)> for Config
{
    type Output = String;

    fn index(&self, key: (&str, &str)) -> &Self::Output {
        &self.values[&(key.0.into(), key.1.into())]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_variables() {
        let conf = Config::read_from_file("test/test.ini").unwrap();

        assert_eq!(conf[("DEFAULT", "var1")], "val1");
        assert_eq!(conf[("Group A", "var2")], "value two");
        assert_eq!(conf[("Group A", "var 3")], "value = three");
    }

    #[test]
    fn merge_configs() {
        let conf = Config::read_from_file("test/test.ini").unwrap()
            .merge_with(Config::read_from_file("test/test2.ini").unwrap());

        assert_eq!(conf[("DEFAULT", "var1")], "val1");
        assert_eq!(conf[("Group A", "var2")], "value two");
        assert_eq!(conf[("Group A", "var 3")], "value = four");
    }

    #[test]
    fn get_default_group() {
        let conf = Config::read_from_file("test/test.ini").unwrap();

        assert_eq!(conf.get("var1"), Some(&"val1".to_string()));
        assert_eq!(conf.get("nothing"), None);
    }

    #[test]
    fn get_group() {
        let conf = Config::read_from_file("test/test.ini").unwrap();

        assert_eq!(
            conf.get_group("Group A", "var2"),
            Some(&"value two".to_string())
        );
        assert_eq!(conf.get_group("Group A", "var1"), None);
    }

    #[test]
    fn set_default_values() {
        let conf = Config::default()
            .set_default("var1", "default value")
            .set_default("some-var", "my-value")
            .merge_with(Config::read_from_file("test/test.ini").unwrap());

        assert_eq!(conf["var1"], "val1");
        assert_eq!(conf["some-var"], "my-value");
    }
}
