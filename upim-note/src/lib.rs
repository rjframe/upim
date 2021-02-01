//! The uPIM Note API.
//!
//! A [Note] is a header and textual document, both UTF-8-encoded. The header
//! contains arbitrary tags and key-value metadata. The header and document are
//! separated by an empty line.
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


#![feature(split_inclusive)]
#![feature(str_split_once)]
#![feature(with_options)]

use std::{
    fs::File,
    io::Write,
    str::FromStr,
};

use anyhow::anyhow;


// TODO: Proper error types, handling.

/** uPIM's note type.

    No interpretation of the metadata is performed. Whether multiple key-value
    pairs is allowed is application-specific.

    A tag must begin with the '@' character, must have at least one character
    following the '@' symbol, and ends with the following ' '; no other name
    requirements exist.

    Key-value metadata must not have an open or closing square brace within its
    content ('[', ']'); keys cannot have a colon character (':'); whether values
    may contain a colon is application-specific.

    The content must be UTF-8.
*/
#[derive(Clone, Debug, Default, Eq, PartialEq, Hash)]
struct Note {
    meta: Vec<Metadata>,
    // Large notes are possible; we may not always want to store the full
    // document in memory -- we could use a wrapper type that sets some maximum
    // buffer, backed by a file.
    content: String,
}

impl FromStr for Note {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut note = Self::default();
        let mut lines = s.split_inclusive('\n');

        for line in &mut lines {
            if line == "\n" { break; }

            for meta in Self::read_metadata_line(line)? {
                note.meta.push(meta);
            }
        }

        note.content = lines.collect();

        Ok(note)
    }
}

impl Note {
    pub fn new(meta: &[Metadata], text: &str) -> Self {
        Self {
            meta: meta.into(),
            content: text.into(),
        }
    }

    pub fn read_from_file(path: &str) -> anyhow::Result<Self> {
        use std::io::{prelude::*, BufReader};

        let mut note = Note::default();
        let mut reader = BufReader::new(File::open(path)?);
        let mut line = String::new();

        while reader.read_line(&mut line)? > 1 {
            for meta in Self::read_metadata_line(line.trim())? {
                note.meta.push(meta);
            }
        }

        reader.read_to_string(&mut note.content)?;

        Ok(note)
    }

    pub fn write_to_file(&self, path: &str) -> std::io::Result<()> {
        let mut file = File::create(path)?;

        // TODO: Place tags on a single line up to 80 chars?
        for meta in &self.meta {
            file.write_all(meta.to_string().as_bytes())?;
            file.write_all(b"\n")?;
        }

        file.write_all(b"\n")?;
        file.write_all(self.content.as_bytes())?;

        Ok(())
    }

    fn read_metadata_line(line: &str) -> anyhow::Result<Vec<Metadata>> {
        assert!(line.len() > 1);
        assert!(line.ends_with('\n'), line.to_string());

        let line = &line[0..line.len()-1];

        if line.starts_with('@') {
            let mut res = vec![];

            for tag in line.split(' ') {
                if tag.is_empty() { continue; }

                if tag.starts_with('@') {
                    if tag.len() == 1 {
                        return Err(anyhow!("Empty tags are invalid."));
                    }

                    res.push(Metadata::Tag(tag.into()));
                } else {
                    return Err(
                        anyhow!("Tag is missing the '@' symbol: {}", tag)
                    );
                }
            }

            Ok(res)
        } else if line.starts_with('[') && line.ends_with(']') {
            let line = &line[1..line.len()-1];

            let banned = |c| { c == '[' || c == ']' };
            if line.find(banned).is_some() {
                return Err(anyhow!(
                    "Metadata cannot contain '[' or ']' within its value: [{}]",
                    line
                ));
            }

            match line.split_once(':') {
                Some((k, v)) => {
                    Ok([Metadata::KV(
                        k.trim().into(),
                        v.trim().into()
                    )].into())
                },
                None => {
                    Err(anyhow!("Invalid key/value metadata line: [{}]", line))
                },
            }
        } else {
            Err(anyhow!("Invalid metadata object: {}", line))
        }
    }
}

/** Supported metadata types in a note. */
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
enum Metadata {
    /** An arbitrary data tag on a note. */
    Tag(String),
    /** Key-value metadata on a note. */
    // For a large number of key-value pairs a hashmap would be more efficient
    // than the vector of tuple's we're using now. We're probably fine but may
    // need to change this in the future.
    KV(String, String),
}

impl std::fmt::Display for Metadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            Self::Tag(s) => s.into(),
            Self::KV(k, v) => format!("[{}: {}]", k, v),
        };
        write!(f, "{}", text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // TODO: Sandbox to test reading and writing notes.

    #[test]
    fn read_tag_meta_line() {
        let val = Note::read_metadata_line("@some-tag\n").unwrap();

        assert_eq!(val.len(), 1);
        assert_eq!(val[0], Metadata::Tag("@some-tag".into()));
    }

    #[test]
    fn read_multiple_tags_meta_line() {
        let val = Note::read_metadata_line("@some-tag @other-tag\n").unwrap();

        assert_eq!(val.len(), 2);
        assert_eq!(val[0], Metadata::Tag("@some-tag".into()));
        assert_eq!(val[1], Metadata::Tag("@other-tag".into()));
    }

    #[test]
    fn tags_must_be_prefixed_with_symbol() {
        assert!(Note::read_metadata_line("@some tag\n").is_err());
    }

    #[test]
    fn read_key_value_meta_line() {
        let val = Note::read_metadata_line("[Key: Value]\n").unwrap();

        assert_eq!(val.len(), 1);
        assert_eq!(val[0], Metadata::KV("Key".into(), "Value".into()));
    }

    #[test]
    fn only_one_kv_is_on_a_line() {
        assert!(Note::read_metadata_line("[k:v] [k:v]\n").is_err());
    }

    #[test]
    fn read_note_with_empty_header() {
        let text = "\nSome text.\n";

        let val = Note::from_str(text).unwrap();
        assert_eq!(val.meta.len(), 0);
        assert_eq!(val.content, "Some text.\n");
    }

    #[test]
    fn read_note_with_empty_content() {
        let text = "@tag\n[some:stuff]\n";

        let val = Note::from_str(text).unwrap();
        assert_eq!(val.meta.len(), 2);
        assert_eq!(val.meta[0], Metadata::Tag("@tag".into()));
        assert_eq!(val.meta[1], Metadata::KV("some".into(), "stuff".into()));
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

        assert!(note.meta.len() == 5);
        assert_eq!(note.meta[0], Metadata::Tag("@some-tag".into()));
        assert_eq!(note.meta[1], Metadata::Tag("@other-tag".into()));
        assert_eq!(note.meta[2], Metadata::Tag("@another-tag".into()));
        assert_eq!(note.meta[3], Metadata::KV("Date".into(), "None".into()));
        assert_eq!(note.meta[4], Metadata::KV("Some".into(), "Thing".into()));
        assert_eq!(
            note.content,
            "Some content goes here.\n\nAnd more stuff.\n"
        );
    }
}
