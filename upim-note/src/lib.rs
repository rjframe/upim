//! The uPIM Note API.
//!
//! A [Note] is a header and textual document, both UTF-8-encoded. The header
//! contains arbitrary tags and key-value attributes. The header and document
//! are separated by an empty line.
//!
//! A Note that begins with an empty line contains an empty header. It is not
//! required to contain an extra new-line for a header-only document.
//!
//! # Example Documents
//!
//! ```text
//! @to-read
//! [Author: Favorite Person]
//! [Title: Some Book]
//! [Suggested by: Other Person]
//!
//! This was recommended to me by Other Person because I like books.
//! ```
//!
//! ```text
//! @website
//! @some-subject @another-subject
//! [Source: www.example.com]
//!
//! # Summary
//!
//! This is a summary of the information at example.com.
//! ```
//!
//! No attempt to interpret the header or content is made by the library; all
//! meaning is determined by the user and/or applications. This provides
//! flexibility by allowing virtually limitless classification and
//! cross-referencing of information, without the need to deal with semantic
//! restrictions of the library.
//!
//! # Potential Future Extensions
//!
//! - The document could be more than just text. The current workaround would be
//!   a header-only document with a [Ref: <url>] to another document.

#![feature(str_split_once)]
#![feature(with_options)]

use std::{
    collections::HashMap,
    fs::File,
    io::Write,
    ops::{Index, IndexMut},
    path::Path,
    str::FromStr,
};

use upim_core::error::FileError;


/// uPIM's note type.
///
/// No interpretation of the metadata is performed. Duplicate keys in the
/// attribute list is allowed; applications that seek to disallow duplicates
/// must validate the keys.
///
/// A tag must begin with the '@' character, must have at least one character
/// following the '@' symbol, and ends with the following space or newline; no
/// other name requirements exist. Duplicate tags are allowed but are only
/// stored once.
///
/// Key-value attributes must not have an open or closing square brace within
/// its content ('[', ']'); keys cannot have a colon character (':'); whether
/// values may contain a colon is application-specific.
///
/// The content must be valid UTF-8.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Note {
    /// Arbitrary data tags on a note.
    tags: Vec<String>,
    /// Key-value attributes on a note.
    map: HashMap<String, String>,
    // Large notes are possible; we may not always want to store the full
    // document in memory -- we could use a wrapper type that sets some maximum
    // buffer, backed by a file.
    content: String,
}

impl FromStr for Note {
    type Err = FileError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut note = Self::default();
        let mut lines = s.split_inclusive('\n');
        let mut cnt = 0;

        // Don't want to fight the borrow checker over ownership of `lines`.
        #[allow(clippy::explicit_counter_loop)]
        for line in &mut lines {
            cnt += 1;
            if line == "\n" { break; }

            match Self::read_metadata_line(line, cnt)? {
                Metadata::Tag(mut vs) => { note.tags.append(&mut vs); },
                Metadata::KV(k, v) => { note.map.insert(k, v); },
            }
        }

        note.content = lines.collect();

        Ok(note)
    }
}

impl Index<&str> for Note {
    type Output = String;

    /// Look up an attribute value by key.
    fn index(&self, key: &str) -> &Self::Output {
        &self.map[key]
    }
}

impl IndexMut<&str> for Note {
    /// Modify attribute value by key.
    fn index_mut(&mut self, key: &str) -> &mut Self::Output {
        if ! self.map.contains_key(key) {
            self.map.insert(key.to_string(), String::new());
        }
        self.map.get_mut(key).unwrap()
    }
}

impl Note {
    pub fn new(tags: &[String], attrs: HashMap<String, String>, text: &str)
    -> Self {
        Self {
            tags: tags.into(),
            map: attrs,
            content: text.into(),
        }
    }

    /// Validate the header of a note at the given path.
    pub fn validate_header(path: &Path) -> Result<(), FileError> {
        use std::io::{prelude::*, BufReader};

        let mut reader = BufReader::new(File::open(path)?);
        let mut line = String::new();
        let mut cnt = 0;

        while reader.read_line(&mut line)? > 1 {
            cnt += 1;
            Self::read_metadata_line(&line, cnt)?;
            line.clear();
        }

        Ok(())
    }

    /// Read the file at the given path and parse it as a `Note`.
    pub fn read_from_file(path: &Path) -> Result<Self, FileError> {
        use std::io::{prelude::*, BufReader};

        let mut note = Note::default();
        let mut reader = BufReader::new(File::open(path)?);
        let mut line = String::new();
        let mut cnt = 0;

        while reader.read_line(&mut line)? > 1 {
            cnt += 1;

            match Self::read_metadata_line(&line, cnt)? {
                Metadata::Tag(mut vs) => { note.tags.append(&mut vs); },
                Metadata::KV(k, v) => { note.map.insert(k, v); },
            }
            line.clear();
        }

        reader.read_to_string(&mut note.content)?;

        Ok(note)
    }

    /// Read a Note header from a file.
    ///
    /// Returns a [Note] with an empty content field.
    pub fn read_header(path: &Path) -> Result<Self, FileError> {
        use std::io::{prelude::*, BufReader};

        let mut note = Note::default();
        let mut reader = BufReader::new(File::open(path)?);
        let mut line = String::new();
        let mut cnt = 0;

        while reader.read_line(&mut line)? > 1 {
            cnt += 1;

            match Self::read_metadata_line(&line, cnt)? {
                Metadata::Tag(mut vs) => { note.tags.append(&mut vs); },
                Metadata::KV(k, v) => { note.map.insert(k, v); },
            }
            line.clear();
        }

        Ok(note)
    }

    /// Save the note to the specified path.
    pub fn write_to_file(&self, path: &Path) -> std::io::Result<()> {
        let mut file = File::create(path)?;

        for tag in &self.tags {
            file.write_all(tag.as_bytes())?;
            file.write_all(b"\n")?;
        }

        for (k, v) in &self.map {
            file.write_all(b"[")?;
            file.write_all(k.as_bytes())?;
            file.write_all(b": ")?;
            file.write_all(v.as_bytes())?;
            file.write_all(b"]\n")?;
        }

        file.write_all(b"\n")?;
        file.write_all(self.content.as_bytes())?;

        Ok(())
    }

    /// Add the given tag to the note.
    ///
    /// If the note already exists, does nothing. If the tag is not prepended
    /// with a '@', it is added.
    pub fn insert_tag(&mut self, tag: &str) {
        let tag = if tag.starts_with('@') {
            tag.into()
        } else {
            format!("@{}", tag)
        };

        if ! self.tags.contains(&tag) {
            self.tags.push(tag);
        }
    }

    /// Remove the specified tag.
    ///
    /// If the tag was present, it is returned. Otherwise returns `None`.
    pub fn remove_tag(&mut self, tag: &str) -> Option<String> {
        if let Some(pos) = self.tags.iter().position(|x| *x == tag) {
            Some(self.tags.remove(pos))
        } else {
            None
        }
    }

    /// Check whether the note contains the specified tag.
    pub fn contains_tag(&self, tag: &str) -> bool {
        self.tags.contains(&tag.to_string())
    }

    /// Retrieve the list of tags on the note.
    pub fn tags(&self) -> &[String] {
        &self.tags
    }

    /// Look up the attribute value matching the given key.
    pub fn get_attribute(&self, key: &str) -> Option<&String> {
        self.map.get(key)
    }

    /// Add or update the specified attribute on the note.
    pub fn set_attribute(&mut self, key: &str, value: &str) {
        self.map.insert(key.into(), value.into());
    }

    pub fn remove_attribute(&mut self, key: &str) -> Option<String> {
        self.map.remove(key)
    }

    /// Check whether the note contains the specified attribute.
    pub fn contains_attribute(&self, key: &str) -> bool {
        self.map.contains_key(key)
    }

    pub fn attribute_keys(&self) -> impl Iterator<Item = &String> {
        self.map.keys()
    }

    pub fn attributes(&self) -> impl Iterator<Item = (&String, &String)> {
        self.map.iter()
    }

    /// Get the note's content (document).
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Erase the note's content.
    pub fn clear_content(&mut self) {
        self.content = String::new();
    }


    fn read_metadata_line(line: &str, line_num: u32,)
    -> Result<Metadata, FileError> {
        assert!(line.len() > 1);
        assert!(line.ends_with('\n'), "{}", line.to_string());

        let line = &line[0..line.len()-1];

        if line.starts_with('@') {
            let mut tags = vec![];

            for tag in line.split(' ') {
                if tag.is_empty() { continue; }

                if tag.starts_with('@') {
                    if tag.len() == 1 {
                        return Err(FileError::Parse {
                            msg: "Empty tags are invalid.".into(),
                            data: "@".into(),
                            line: line_num,
                        });
                    }

                    tags.push(tag.into());
                } else {
                    return Err(FileError::Parse {
                        msg: "Tag is missing the '@' symbol".into(),
                        data: tag.into(),
                        line: line_num,
                    });
                }
            }

            Ok(Metadata::Tag(tags))
        } else if line.starts_with('[') && line.ends_with(']') {
            let line = &line[1..line.len()-1];

            let banned = |c| { c == '[' || c == ']' };
            if line.find(banned).is_some() {
                return Err(FileError::Parse {
                    msg: "Key-value pairs cannot contain '[' or ']'".into(),
                    data: line.into(),
                    line: line_num,
                });
            }

            match line.split_once(':') {
                Some((k, v)) => {
                    Ok(Metadata::KV(
                        k.trim().into(),
                        v.trim().into()
                    ))
                },
                None => {
                    Err(FileError::Parse {
                        msg: "Invalid key/value metadata line".into(),
                        data: line.into(),
                        line: line_num,
                    })
                },
            }
        } else {
            Err(FileError::Parse {
                msg: "Invalid metadata object".into(),
                data: line.into(),
                line: line_num,
            })
        }
    }
}

/// Supported metadata types in a note.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
enum Metadata {
    Tag(Vec<String>),
    #[allow(clippy::upper_case_acronyms)]
    KV(String, String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_tag_meta_line() {
        if let Metadata::Tag(vs) =
            Note::read_metadata_line("@some-tag\n", 1).unwrap()
        {
            assert_eq!(vs.len(), 1);
            assert_eq!(vs[0], "@some-tag");
        } else {
            panic!();
        }
    }

    #[test]
    fn read_multiple_tags_meta_line() {
        if let Metadata::Tag(vs)=
            Note::read_metadata_line("@some-tag @other-tag\n", 1).unwrap()
        {
            assert_eq!(vs.len(), 2);
            assert_eq!(vs[0], "@some-tag");
            assert_eq!(vs[1], "@other-tag");
        } else {
            panic!();
        }
    }

    #[test]
    fn tags_must_be_prefixed_with_symbol() {
        assert!(Note::read_metadata_line("@some tag\n", 1).is_err());
    }

    #[test]
    fn read_key_value_meta_line() {
        if let Metadata::KV(k, v) =
            Note::read_metadata_line("[Key: Value]\n", 1).unwrap()
        {
            assert_eq!(k, "Key");
            assert_eq!(v, "Value");
        }
    }

    #[test]
    fn only_one_kv_is_on_a_line() {
        assert!(Note::read_metadata_line("[k:v] [k:v]\n", 1).is_err());
    }

    #[test]
    fn read_note_with_empty_header() {
        let text = "\nSome text.\n";

        let val = Note::from_str(text).unwrap();
        assert_eq!(val.tags.len(), 0);
        assert_eq!(val.map.len(), 0);
        assert_eq!(val.content, "Some text.\n");
    }

    #[test]
    fn fail_to_read_note_with_missing_header() {
        let text = "Some text.\n";
        assert!(Note::from_str(text).is_err());
    }

    #[test]
    fn read_note_with_empty_content() {
        let text = "@tag\n[some:stuff]\n";

        let val = Note::from_str(text).unwrap();
        assert_eq!(val.tags.len(), 1);
        assert_eq!(val.map.len(), 1);
        assert_eq!(val.tags[0], "@tag");
        assert_eq!(val.map["some"], "stuff");
        assert_eq!(val.content, "");
    }

    #[test]
    fn read_full_note() {
        let text = "\
        @some-tag @other-tag\n\
        @another-tag\n\
        [Date: None]\n\
        [Some: Thing]\n\
        \n\
        Some content goes here.\n\
        \n\
        And more stuff.\n\
        ";

        let note = Note::from_str(text).unwrap();

        assert_eq!(note.tags.len(), 3);
        assert_eq!(note.map.len(), 2);
        assert_eq!(note.tags[0], "@some-tag");
        assert_eq!(note.tags[1], "@other-tag");
        assert_eq!(note.tags[2], "@another-tag");
        assert_eq!(note.map["Date"], "None");
        assert_eq!(note.map["Some"], "Thing");
        assert_eq!(
            note.content,
            "Some content goes here.\n\nAnd more stuff.\n"
        );
    }

    #[test]
    fn lookup_attribute_by_key() {
        let text = "\
        [Date: None]\n\
        [Some: Thing]\n\
        ";

        let note = Note::from_str(text).unwrap();
        assert_eq!(note.get_attribute("Date"), Some(&String::from("None")));
        assert_eq!(note.get_attribute("Some"), Some(&String::from("Thing")));
        assert_eq!(note["Date"], "None");
        assert_eq!(note["Some"], "Thing");
    }

    #[test]
    fn create_and_modify_attributes_by_key() {
        let text = "\
        [Date: None]\n\
        [Some: Thing]\n\
        ";

        let mut note = Note::from_str(text).unwrap();
        note["Date"] = "January 1".into();
        note["Year"] = "2000".into();

        assert_eq!(note["Date"], "January 1");
        assert_eq!(note["Some"], "Thing");
        assert_eq!(note["Year"], "2000");
    }

    #[test]
    fn note_contains_tag() {
        let text = "@tag1 @tag2\n";
        let note = Note::from_str(text).unwrap();

        assert!(note.contains_tag("@tag1"));
        assert!(note.contains_tag("@tag2"));
        assert!(! note.contains_tag("@tag3"));
    }

    #[test]
    fn note_add_tag() {
        let text = "@tag1\n";
        let mut note = Note::from_str(text).unwrap();

        note.insert_tag("@tag2");
        note.insert_tag("tag3");

        assert!(note.contains_tag("@tag1"));
        assert!(note.contains_tag("@tag2"));
        assert!(note.contains_tag("@tag3"));
    }

    #[test]
    fn note_remove_tag() {
        let text = "@tag1 @tag2\n";
        let mut note = Note::from_str(text).unwrap();

        assert_eq!(note.remove_tag("@tag2"), Some("@tag2".to_string()));
        assert!(note.contains_tag("@tag1"));
        assert!(! note.contains_tag("@tag2"));
    }

    #[test]
    fn note_list_tags() {
        let text = "@tag1 @tag2\n";
        let note = Note::from_str(text).unwrap();

        assert_eq!(note.tags(), ["@tag1".to_string(), "@tag2".to_string()]);
    }

    #[test]
    fn note_clear_content_data() {
        let text = "\
        @some-tag @other-tag\n\
        @another-tag\n\
        [Date: None]\n\
        [Some: Thing]\n\
        \n\
        Some content goes here.\n\
        \n\
        And more stuff.\n\
        ";

        let mut note = Note::from_str(text).unwrap();
        assert!(! note.content().is_empty());
        note.clear_content();
        assert!(note.content().is_empty());
    }
}
