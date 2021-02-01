#![feature(split_inclusive)]
#![feature(str_split_once)]

use std::str::FromStr;

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

    pub fn from_file(path: &str) -> anyhow::Result<Self> {
        use std::{
            fs::File,
            io::{prelude::*, BufReader},
        };

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

#[cfg(test)]
mod tests {
    use super::*;

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
