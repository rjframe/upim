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

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    ops::Index,
    env,
};

use super::{
    error::FileError,
    uniq::Uniq,
};


#[cfg(target_os = "macos")]
static BUNDLE_ID: &str = "us.simplifysystems.uPIM";

/// Read the standard uPIM configuration.
///
/// Configurations are read from the following paths, in the following order:
///
/// On macOS:
///
/// 1. `/Library/Application Support/us.simplifysystems.uPIM/upim.conf`
/// 2. `/etc/upim/upim.conf`
/// 3. `~/Library/Application Support/us.simplifysystems.uPIM/upim.conf`
/// 4. `$HOME/.config/upim/upim.conf`
/// 5. `<current working directory>/.upim.conf`
///
/// It is recommended to use #1 and #3 or #2 and #4, but not to mix the macOS
/// and UNIX standard locations.
///
/// On other UNIX-like operating systems:
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
pub fn read_upim_configuration() -> Result<Config, Vec<FileError>> {
    let conf_files = get_upim_configuration_paths().unwrap_or_default();
    let mut conf = Config::default();
    let mut errors = vec![];

    for file in conf_files.iter() {
        match Config::read_from_file(file) {
            Ok(c) => conf = conf.merge_with(c),
            Err(mut e) => errors.append(&mut e),
        };
    }

    if errors.is_empty() {
        Ok(conf)
    } else {
        Err(errors)
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

/// Get the path to the first application configuration file discovered.
///
/// # Parameters
///
/// - name: The name (without file extension) of the configuration file to
///         search for.
pub fn find_application_configuration(name: &str) -> Option<PathBuf> {
    let ext = if cfg!(windows) {
        "ini"
    } else {
        "conf"
    };

    let mut paths = get_upim_configuration_dirs().unwrap_or_default();

    paths.iter_mut()
        .find_map(|p| {
            p.push(name);
            p.set_extension(ext);
            p.exists().then_some(p.clone())
        })
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
    ///
    /// # Returns
    ///
    /// Returns the configuration file if successfully read; otherwise returns a
    /// list of errors that occurred while reading or parsing the file.
    pub fn read_from_file(path: &Path) -> Result<Self, Vec<FileError>> {
        use std::{
            fs::File,
            io::{prelude::*, BufReader},
        };

        let f = match File::open(path) {
            Ok(f) => f,
            Err(e) => return Err(vec![e.into()]),
        };

        let mut reader = BufReader::new(f);
        let mut line = String::new();
        let mut cnt = 0;

        let mut errors = vec![];
        let mut map = HashMap::new();
        let mut group = String::from("DEFAULT");

        loop {
            match reader.read_line(&mut line) {
                Ok(len) => if len == 0 { break; },
                Err(e) => {
                    errors.push(e.into());
                    continue;
                }
            };

            cnt += 1;
            line = line.trim().into();
            if line.is_empty() { continue; }

            if line.starts_with(';') {
                line.clear();
                continue;
            } else if line.starts_with('[') {
                if line.ends_with(']') {
                    group = line[1..line.len()-1].trim().into();
                } else {
                    errors.push(FileError::Parse {
                        msg: "Missing closing bracket for group name".into(),
                        data: line.to_owned(),
                        line: cnt,
                    });
                }
            } else if let Some((var, val)) = line.split_once('=') {
                // We'll allow empty values, but not variables.
                let var = var.trim_end().to_string();

                if var.is_empty() {
                    errors.push(FileError::Parse {
                        msg: "Assignment requires a variable name".into(),
                        data: line.to_owned(),
                        line: cnt,
                    });
                } else {
                    map.insert(
                        (group.clone(), var),
                        val.trim_start().to_string()
                    );
                }
            } else {
                errors.push(FileError::Parse {
                    msg: "Expected a variable assignment".into(),
                    data: line.to_owned(),
                    line: cnt,
                });
            }
            line.clear();
        }

        if errors.is_empty() {
            Ok(Self { values: map })
        } else {
            Err(errors)
        }
    }

    /// Write this configuration to the given file. If the file exists, it is
    /// replaced with the contents of this configuration.
    pub fn write_to_file(&self, path: &Path) -> Result<(), FileError> {
        use std::{
            io::Write as _,
            fs::File,
        };

        let mut file = File::create(path)?;

        for group in self.groups() {
            writeln!(file, "[{}]", group)?;

            for var in self.variables_in_group(&group) {
                writeln!(
                    file,
                    "{} = {}",
                    var,
                    self[(group.as_str(), var.as_str())]
                )?;
            }
        }

        Ok(())
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
    /// use std::path::Path;
    /// use upim_core::Config;
    ///
    /// // If test.ini includes `var1 = val1` but does not have `some-var`:
    /// let conf = Config::default()
    ///     .set("DEFAULT", "var1", "default value")
    ///     .set("DEFAULT", "some-var", "my-value")
    ///     .merge_with(
    ///         Config::read_from_file(Path::new("test/test.ini")).unwrap()
    ///     );
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

    /// Get the list of groups in the configuration file.
    pub fn groups(&self) -> impl Iterator<Item = &String> {
        self.values.keys().uniq().map(|k| &k.0)
    }

    /// Get the list of variables set in the specified group.
    pub fn variables_in_group<'a>(&'a self, group: &'a str)
    -> impl Iterator<Item = &'a String> {
        self.values.keys()
            .filter_map(move |k| {
                if k.0 == group { Some(&k.1) } else { None }
            })
    }

    /// Retrieve the value of the specified variable within the DEFAULT group,
    /// or `None` if it is not set.
    pub fn get_default(&self, variable: &str) -> Option<&String> {
        self.values.get(&("DEFAULT".into(), variable.into()))
    }

    /// Retrieve the value of the specified variable within the specified group,
    /// or `None` if it is not set.
    pub fn get(&self, group: &str, variable: &str) -> Option<&String> {
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
        dir.push("upim.conf");
        if dir.exists() {
            paths.push(dir.to_path_buf());
        }
    }

    let mut pbuf = env::current_dir()
        .map_or_else(|_| PathBuf::default(), |v| v);
    pbuf.push(".upim.conf");

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
        dir.push("upim.ini");
        if dir.exists() {
            paths.push(dir.to_path_buf());
        }
    }

    let mut pbuf = env::current_dir()
        .map_or_else(|_e| PathBuf::default(), |v| v);
    pbuf.push(r"upim.ini");

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
    let mut paths = vec![];

    #[cfg(target_os = "macos")]
    if Path::new(concat!("/Library/Application Support/", BUNDLE_ID)).exists() {
        paths.push(
            PathBuf::from(concat!("/Library/Application Support/", BUNDLE_ID))
        );
    }

    if Path::new("/etc/upim").exists() {
        paths.push(PathBuf::from("/etc/upim"));
    }

    let home = env::var_os("HOME");

    #[cfg(target_os = "macos")]
    if let Some(home) = home {
        let p = home.join("Library/Application Support").join(BUNDLE_ID);
        if p.exists() {
            paths.push(PathBuf::from(p));
        }
    }

    let path = if let Some(p) = env::var_os("XDG_CONFIG_HOME") {
        Path::new(&p).join("upim")
    } else if let Some(p) = home {
        Path::new(&p).join(".config/upim")
    } else {
        PathBuf::default()
    };

    if path.exists() {
        paths.push(path);
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
    let mut paths = vec![];

    if let Some(path) = env::var_os("PROGRAMDATA") {
        paths.push(PathBuf::from(path).join("uPIM"));
    }

    if let Some(path) = env::var_os("APPDATA") {
        paths.push(PathBuf::from(path).join("uPIM"));
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
        let conf = Config::read_from_file(Path::new("test/test.ini")).unwrap();

        assert_eq!(conf[("DEFAULT", "var1")], "val1");
        assert_eq!(conf[("Group A", "var2")], "value two");
        assert_eq!(conf[("Group A", "var 3")], "value = three");
    }

    #[test]
    fn merge_configs() {
        let conf = Config::read_from_file(Path::new("test/test.ini")).unwrap()
            .merge_with(Config::read_from_file(Path::new("test/test2.ini"))
                .unwrap());

        assert_eq!(conf[("DEFAULT", "var1")], "val1");
        assert_eq!(conf[("Group A", "var2")], "value two");
        assert_eq!(conf[("Group A", "var 3")], "value = four");
    }

    #[test]
    fn write_to_file() {
        use std::{
            fs::remove_file,
            env,
        };

        let mut path = env::temp_dir();
        path.push("writing_test_config_file");
        path.set_extension("txt");

        let _ = remove_file(&path);

        let conf = Config::default()
            .set_default("var1", "value")
            .set("Some Group", "my variable", "my value");

        conf.write_to_file(&path).unwrap();

        let read_conf = Config::read_from_file(&path).unwrap();
        assert_eq!(read_conf.get_default("var1"), Some(&"value".to_string()));
        assert_eq!(
            read_conf.get("Some Group", "my variable"),
            Some(&"my value".to_string())
        );

        let _ = remove_file(&path);
    }

    #[test]
    fn nonexistent_file_is_err() {
        let conf = Config::read_from_file(Path::new("nopath/notexist.conf"));
        assert!(conf.is_err());
    }

    #[test]
    fn get_default_group() {
        let conf = Config::read_from_file(Path::new("test/test.ini")).unwrap();

        assert_eq!(conf.get_default("var1"), Some(&"val1".to_string()));
        assert_eq!(conf.get_default("nothing"), None);
    }

    #[test]
    fn get_group() {
        let conf = Config::read_from_file(Path::new("test/test.ini")).unwrap();

        assert_eq!(conf.get("Group A", "var2"), Some(&"value two".to_string()));
        assert_eq!(conf.get("Group A", "var1"), None);
    }

    #[test]
    fn get_nonexistent_group_is_none() {
        let conf = Config::read_from_file(Path::new("test/test.ini")).unwrap();

        assert!(conf.get("Not a group", "var1").is_none());
    }

    #[test]
    fn set_default_values() {
        let conf = Config::default()
            .set_default("var1", "default value")
            .set_default("some-var", "my-value")
            .merge_with(Config::read_from_file(Path::new("test/test.ini"))
                .unwrap());

        assert_eq!(conf["var1"], "val1");
        assert_eq!(conf["some-var"], "my-value");
    }

    #[test]
    fn collect_all_parse_errors() {
        let conf = Config::read_from_file(Path::new("test/invalid.ini"));
        let errs = conf.unwrap_err();
        let mut errs = errs.iter();

        match errs.next() {
            Some(FileError::Parse { msg, data, line }) => {
                assert!(msg.contains("variable assignment"));
                assert_eq!(data, "some variable");
                assert_eq!(*line, 3);
            },
            _ => panic!("Expected a FileError::Parse"),
        }

        match errs.next() {
            Some(FileError::Parse { msg, data, line }) => {
                assert!(msg.contains("variable name"));
                assert_eq!(data, "= some value");
                assert_eq!(*line, 5);
            },
            _ => panic!("Expected a FileError::Parse"),
        }

        match errs.next() {
            Some(FileError::Parse { msg, data, line }) => {
                assert!(msg.contains("closing bracket"));
                assert_eq!(data, "[Bad Group");
                assert_eq!(*line, 7);
            },
            _ => panic!("Expected a FileError::Parse"),
        }

        match errs.next() {
            Some(FileError::Parse { msg, data, line }) => {
                assert!(msg.contains("variable assignment"));
                assert_eq!(data, "# Bad comment");
                assert_eq!(*line, 9);
            },
            _ => panic!("Expected a FileError::Parse"),
        }

        assert!(errs.next().is_none());
    }
}
