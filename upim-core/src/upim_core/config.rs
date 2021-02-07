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
    path::{Path, PathBuf},
    ops::Index,
    env,
};

use super::error::FileError;


// TODO: Write to file.
// TODO: We should probably have a separate directory setup for macOS.

/// Read the standard uPIM configuration.
///
/// Configurations are read from the following paths, in the following order:
///
/// On UNIX-like operating systems:
///
/// 1. `/etc/upim/upim.conf`
/// 2. `$XDG_CONFIG_HOME/upim/upim.conf` XOR `$HOME/.config/upim/upim.conf`
/// 3. `<current working directory>/.upim.conf`
///
/// On Windows:
///
/// 1. `%PROGRMDATA%\uPIM\upim.ini`
/// 2. `%APPDATA\uPIM\upim.ini`
/// 3. `<current working directory>\upim.ini`
///
/// Values set in later files override the earlier values, so the priority is in
/// the reverse order of the list above.
///
/// Applications built upon uPIM may place their own configuration files within
/// a `upim` configuration directory but will need to read that configuration
/// via the [Config] object rather than this function.
///
/// # Returns
///
/// If no configuration files were found, returns [Config::default].
///
/// Returns `Ok(Config)` if all discovered configuration files were successfully
/// read. Otherwise, returns `Err(Config)` containing the settings from all
/// successfully-read configuration files.
///
/// This function will be updated to report what errors occured in the failure
/// case.
pub fn read_upim_configuration() -> Result<Config, Config> {
    let conf_files = get_upim_configuration_paths().unwrap_or_default();
    let mut conf = Config::default();
    let mut err_occured = false;

    // TODO: we'll want .iter().try_for_each().collect::Result<...>() instead.
    for file in conf_files.iter() {
        let c = Config::read_from_file(file.to_str().unwrap_or_default());

        if let Ok(c) = c {
            conf = conf.merge_with(c);
        } else {
            err_occured = true;
        }
    }

    if err_occured {
        Err(conf)
    } else {
        Ok(conf)
    }
}

/// Find and return the paths to the 'upim' directories discovered.
///
/// Only returns directories for uPIM configuration files; not top-level
/// configuration directories (for example, if the only configuration file on
/// the system is `/etc/upim.conf` this will return `None`; if there is an
/// `/etc/upim/upim.conf` then this will return a [PathBuf] to `/etc/upim`).
///
/// This is most useful for applications that want to drop a configuration file
/// into a uPIM configuration directory.
///
/// If all directories that contain a uPIM configuration file are desired, call
/// [get_upim_configuration_paths] and call [PathBuf::pop] on each path
/// returned.
///
/// See the documentation for [read_upim_configuration] for the possible
/// locations of the configuration files.
pub fn get_upim_configuration_dirs() -> Option<Vec<PathBuf>> {
    #![allow(unreachable_code)]

    #[cfg(windows)]
    return get_windows_dirs();

    #[cfg(unix)]
    return get_unixy_dirs();

    panic!();
}

/// Find and return the paths to the configuration files discovered.
///
/// See the documentation for [read_upim_configuration] for the possible
/// locations of the configuration files.
pub fn get_upim_configuration_paths() -> Option<Vec<PathBuf>> {
    #![allow(unreachable_code)]

    #[cfg(windows)]
    return get_windows_paths();

    #[cfg(unix)]
    return get_unixy_paths();

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
        // TODO: needs to be unique
        self.values.keys().map(|k| k.0.clone()).collect()
    }

    // TODO: Vec<&str>? Iterator?
    // TODO: Return var/val pair iterator?
    /// Get the list of variables set in the specified group.
    pub fn variables_in_group(&self, group: &str) -> Vec<String> {
        self.values.keys()
            .filter_map(|k| {
                if k.0 == group { Some(k.1.clone()) } else { None }
            })
            .collect()
    }

    // TODO: Rename these: get_default and get to match the sets:

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

/// See the documentation for [read_upim_configuration] for the possible
/// locations of the configuration files.
#[allow(dead_code)]
fn get_unixy_paths() -> Option<Vec<PathBuf>> {
    let mut paths: Vec<PathBuf> = vec![];
    let mut dirs = get_unixy_dirs().unwrap_or_default();

    for dir in dirs.iter_mut() {
        dir.push("/upim.conf");
        if dir.exists() {
            paths.push(dir.to_path_buf());
        }
    }

    let mut pbuf = env::current_dir()
        .map_or_else(|_| PathBuf::default(), |v| v);
    pbuf.push("/.upim.conf");

    if pbuf.as_path().exists() {
        paths.push(pbuf);
    }

    if ! paths.is_empty() {
        Some(paths)
    } else {
        None
    }
}

/// See the documentation for [read_upim_configuration] for the possible
/// locations of the configuration files.
#[allow(dead_code)]
fn get_windows_paths() -> Option<Vec<PathBuf>> {
    let mut paths = vec![];
    let mut dirs = get_windows_dirs().unwrap_or_default();

    for dir in dirs.iter_mut() {
        dir.push("/upim.ini");
        if dir.exists() {
            paths.push(dir.to_path_buf());
        }
    }

    let mut pbuf = env::current_dir()
        .map_or_else(|_e| PathBuf::default(), |v| v);
    pbuf.push(r"\upim.ini");

    if pbuf.as_path().exists() {
        paths.push(pbuf);
    }

    if paths.is_empty() {
        Some(paths)
    } else {
        None
    }
}

/// See the documentation for [read_upim_configuration] for the possible
/// locations of the configuration files.
#[allow(dead_code)]
fn get_unixy_dirs() -> Option<Vec<PathBuf>> {
    use std::ffi::OsString;

    let mut paths = vec![];

    if Path::new("/etc/upim").exists() {
        paths.push(PathBuf::from("/etc/upim"));
    }

    let path = if let Some(mut p) = env::var_os("XDG_CONFIG_HOME") {
        p.push(OsString::from("/upim"));
        p
    } else if let Some(mut p) = env::var_os("HOME") {
        p.push(OsString::from("/.config/upim"));
        p
    } else {
        OsString::from("")
    };

    if Path::new(&path).exists() {
        paths.push(PathBuf::from(path));
    }

    if ! paths.is_empty() {
        Some(paths)
    } else {
        None
    }
}

/// See the documentation for [read_upim_configuration] for the possible
/// locations of the configuration files.
#[allow(dead_code)]
fn get_windows_dirs() -> Option<Vec<PathBuf>> {
    use std::ffi::OsString;

    let mut paths = vec![];

    if let Some(mut path) = env::var_os("PROGRAMDATA") {
        path.push(OsString::from(r"\uPIM"));
        paths.push(PathBuf::from(path));
    }

    if let Some(mut path) = env::var_os("APPDATA") {
        path.push(OsString::from(r"\uPIM"));
        paths.push(PathBuf::from(path));
    }

    if paths.is_empty() {
        Some(paths)
    } else {
        None
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
