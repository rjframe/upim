use std::{
    collections::hash_map::Keys as Groups,
    path::Path,
    str::FromStr as _,
};

use anyhow::anyhow;
use multimap::MultiMap;
use walkdir::WalkDir;

use upim_note::Note;

use crate::filter::{Condition, FilterOp, Query};

/// Data structure to store the contact information for a person or group.
///
/// Very few limitations are placed on the type of data or organization of that
/// data.
///
/// # Standard Fields
///
/// Standard Fields are recognized by the Contact API in some way. All fields
/// are optional, but there must be a name combination:
///
/// * Name or Given/First Name plus Family/Last Name.
///
/// The following fields have a standard meaning:
///
/// * Name (or Full Name): used to identify the contact.
/// * Given Name, First Name: combined with a family/last name to create a Name.
/// * Family Name, Last Name: combined with a given/first name to create a Name.
///
/// (TODO: Finish documenting: groups)
#[derive(Debug)]
pub struct Contact {
    tags: Vec<String>,
    info: MultiMap<String, Note>,
}

impl Contact {
    /// Create a Contact from the given [Note].
    pub fn new(contact: Note) -> anyhow::Result<Self> {
        // A Contact is stored as a Note, where the content, if present, is a
        // Note, recursively. The final Note may have any textual content.

        let mut notes = vec![];
        let mut parent = contact;
        let tags = parent.tags().to_vec();

        // The first note's tags belong to the contact, so we'll remove them
        // from the note itself.
        for tag in &tags {
            parent.remove_tag(&tag);
        }

        loop {
            if let Ok(n) = Note::from_str(parent.content()) {
                // If the child is a note, we no longer care about the content.
                parent.clear_content();
                notes.push(parent);
                parent = n;

                // An empty note is valid so we need to duplicate the else block
                // below. TODO: Refactor this.
                if parent.content().is_empty() {
                    notes.push(parent);
                    break;
                }
            } else {
                notes.push(parent);
                break;
            }
        }

        let mut info = MultiMap::new();
        let mut last_group = String::from("default"); // Key for the first note.

        for note in notes.iter() {
            if let Some(tag) = note.tags().first() {
                // Remove the leading ampersand.
                last_group = tag[1..].to_lowercase();
            }
            info.insert(last_group.clone(), note.clone());
        }

        let contact = Self { tags: tags.to_vec(), info };

        if contact.name().is_some() {
            Ok(contact)
        } else {
            Err(anyhow!("No name provided in contact"))
        }
    }

    /// Load the file at the given path as a Contact.
    pub fn new_from_file(path: &Path) -> anyhow::Result<Self> {
        Self::new(Note::read_from_file(path)?)
    }

    /// Get the name of this contact.
    ///
    /// Returns the first attribute(s) of:
    ///
    /// * Name or Full Name
    /// * (Given Name or First Name) and (Family Name or Last Name)
    ///
    /// If only the given/first or family/last name is present, returns what we
    /// have.
    pub fn name(&self) -> Option<String> {
        let def = self.info.get("default").unwrap();

        match def.get_attribute("Name")
            .or_else(|| def.get_attribute("Full Name"))
        {
            Some(v) => Some(v.into()),
            None => {
                let given = def.get_attribute("Given Name")
                    .or_else(|| def.get_attribute("First Name"));
                let family = def.get_attribute("Family Name")
                    .or_else(|| def.get_attribute("Last Name"));

                if given.or(family).is_some() {
                    Some(format!(
                        "{} {}",
                        given.cloned().unwrap_or_default(),
                        family.cloned().unwrap_or_default()
                    ).trim().into())
                } else {
                    None
                }
            },
        }
    }

    /// Get the value of a field from the default information group.
    pub fn get_field(&self, name: &str) -> Option<&String> {
        self.get_field_from("default", name)
    }

    /// Get the value of a field from the specified information group.
    pub fn get_field_from(&self, group: &str, name: &str) -> Option<&String> {
        self.info.get(&group.to_lowercase())
            .and_then(|g| g.get_attribute(name))
    }

    /// Return an iterator of the groups defined by the Contact.
    pub fn groups(&self) -> Groups<String, Vec<Note>> {
        self.info.keys()
    }

    pub fn matches(&self, condition: &Condition) -> bool {
        match condition {
            Condition::All => true,
            Condition::Filter(field, ref op, value) => {
                let (group, field) = field.split_once(':')
                    .unwrap_or(("default", field));

                if let Some(info) = self.info.get(&group.to_lowercase()) {
                    let attr = if let Some(f) = info.get_attribute(field) {
                        f
                    } else {
                        // If the operator is `Not` and the field doesn't exist,
                        // count that as not matching.
                        return *op == FilterOp::Not;
                    };

                    // TODO: On parse errors, return an error instead of false?
                    match op {
                        FilterOp::EqualTo => attr == value,
                        FilterOp::LessThan => {
                            let attr = if let Ok(a) = attr.parse::<f32>() {
                                a
                            } else {
                                return false;
                            };

                            value.parse::<f32>()
                                .map(|v| attr < v)
                                .unwrap_or(false)
                        },
                        FilterOp::LessEq => {
                            let attr = if let Ok(a) = attr.parse::<f32>() {
                                a
                            } else {
                                return false;
                            };

                            value.parse::<f32>()
                                .map(|v| attr <= v)
                                .unwrap_or(false)
                        },
                        FilterOp::GreaterThan => {
                            let attr = if let Ok(a) = attr.parse::<f32>() {
                                a
                            } else {
                                return false;
                            };

                            value.parse::<f32>()
                                .map(|v| attr > v)
                                .unwrap_or(false)
                        },
                        FilterOp::GreaterEq => {
                            let attr = if let Ok(a) = attr.parse::<f32>() {
                                a
                            } else {
                                return false;
                            };

                            value.parse::<f32>()
                                .map(|v| attr >= v)
                                .unwrap_or(false)
                        },
                        FilterOp::Not => attr != value,
                    }
                } else {
                    false
                }
            },
            Condition::Function(ref func) => {
                todo!();
            },
            Condition::And(inner) => {
                let (lhs, rhs): &(Condition, Condition) = &**inner;
                self.matches(lhs) && self.matches(rhs)
            },
            Condition::Or(inner) => {
                let (lhs, rhs): &(Condition, Condition) = &**inner;
                self.matches(lhs) || self.matches(rhs)
            },
        }
    }
}

pub fn read_contacts(path: &Path, filter: Query) -> anyhow::Result<Vec<Contact>>
{
    if ! path.is_dir() {
        return Err(anyhow!("The contacts collection must be a directory"));
    }

    let mut contacts = vec![];

    for entry in WalkDir::new(path).min_depth(1).follow_links(true) {
        match entry {
            Err(e) => {
                if e.loop_ancestor().is_some() {
                    continue;
                } else {
                    return Err(anyhow::Error::new(e));
                }
            },
            Ok(entry) => {
                if entry.file_type().is_file() {
                    let contact = Contact::new_from_file(entry.path())?;

                    if contact.matches(&filter.condition) {
                        contacts.push(contact)
                    }
                }
            }
        }
    }

    Ok(contacts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_contact_name() {
        let text = "\
        [Name: Favorite Person]\n\
        [Phone: 123-456]\n\
        ";

        let contact = Contact::new(Note::from_str(text).unwrap()).unwrap();
        assert_eq!(contact.name().unwrap(), "Favorite Person");
    }

    #[test]
    fn simple_contact_full_name() {
        let text = "\
        [Full Name: Favorite Person]\n\
        [Phone: 123-456]\n\
        ";

        let contact = Contact::new(Note::from_str(text).unwrap()).unwrap();
        assert_eq!(contact.name().unwrap(), "Favorite Person");
    }

    #[test]
    fn merge_given_family_name() {
        let text = "\
        [Given Name: Favorite]\n\
        [Family Name: Person]\n\
        [Phone: 123-456]\n\
        ";

        let contact = Contact::new(Note::from_str(text).unwrap()).unwrap();
        assert_eq!(contact.name().unwrap(), "Favorite Person");
    }

    #[test]
    fn merge_first_last_name() {
        let text = "\
        [First Name: Favorite]\n\
        [Last Name: Person]\n\
        [Phone: 123-456]\n\
        ";

        let contact = Contact::new(Note::from_str(text).unwrap()).unwrap();
        assert_eq!(contact.name().unwrap(), "Favorite Person");
    }

    #[test]
    fn new_contact_is_error_with_no_name() {
        let text = "\
        [Phone: 123-456]\n\
        ";

        assert!(Contact::new(Note::from_str(text).unwrap()).is_err());
    }

    #[test]
    fn get_field() {
        let text = "\
        [Name: Favorite Person]\n\
        [Phone: 123-456]\n\
        ";

        let contact = Contact::new(Note::from_str(text).unwrap()).unwrap();
        assert_eq!(contact.get_field("Phone").unwrap(), "123-456");
    }

    #[test]
    fn get_field_from_group() {
        let text = "\
        [Name: Favorite Person]\n\
        \n\
        @employer\n\
        [Name: Some Company]\n\
        [Address: 123 Somewhere]\n\
        ";

        let contact = Contact::new(Note::from_str(text).unwrap()).unwrap();
        assert_eq!(
            contact.get_field_from("employer", "Name").unwrap(),
            "Some Company"
        );
        assert_eq!(
            contact.get_field_from("employer", "Address").unwrap(),
            "123 Somewhere"
        );
    }

    #[test]
    fn group_list() {
        let text = "\
        [Name: Favorite Person]\n\
        \n\
        @employer\n\
        [Name: Some Company]\n\
        ";

        let contact = Contact::new(Note::from_str(text).unwrap()).unwrap();
        let groups: Vec<&String> = contact.groups().collect();

        assert!(groups.contains(&&String::from("default")));
        assert!(groups.contains(&&String::from("employer")));
        assert_eq!(groups.len(), 2);
    }

    #[test]
    fn filter_equal() {
        let text = "\
        [Name: Favorite Person]\n\
        [Num: 123]\n\
        ";

        let contact = Contact::new(Note::from_str(text).unwrap()).unwrap();

        let cond_true1 = Condition::Filter(
            "Name".into(),
            FilterOp::EqualTo,
            "Favorite Person".into()
        );
        let cond_true2 = Condition::Filter(
            "Num".into(),
            FilterOp::EqualTo,
            "123".into()
        );

        let cond_false1 = Condition::Filter(
            "Name".into(),
            FilterOp::EqualTo,
            "Other".into()
        );
        let cond_false2 = Condition::Filter(
            "Num".into(),
            FilterOp::EqualTo,
            "12".into()
        );
        let cond_false3 = Condition::Filter(
            "Stuff".into(),
            FilterOp::EqualTo,
            "a".into()
        );

        assert!(contact.matches(&cond_true1));
        assert!(contact.matches(&cond_true2));
        assert!(! contact.matches(&cond_false1));
        assert!(! contact.matches(&cond_false2));
        assert!(! contact.matches(&cond_false3));
    }

    #[test]
    fn filter_less_than() {
        let text = "\
        [Name: Favorite Person]\n\
        [Num: 123]\n\
        ";

        let contact = Contact::new(Note::from_str(text).unwrap()).unwrap();

        let cond_true = Condition::Filter(
            "Num".into(),
            FilterOp::LessThan,
            "200".into()
        );

        let cond_false1 = Condition::Filter(
            "Name".into(),
            FilterOp::LessThan,
            "Other".into()
        );
        let cond_false2 = Condition::Filter(
            "Num".into(),
            FilterOp::LessThan,
            "12".into()
        );
        let cond_false3 = Condition::Filter(
            "Stuff".into(),
            FilterOp::LessThan,
            "5".into()
        );

        assert!(contact.matches(&cond_true));
        assert!(! contact.matches(&cond_false1));
        assert!(! contact.matches(&cond_false2));
        assert!(! contact.matches(&cond_false3));
    }

    #[test]
    fn filter_less_equal() {
        let text = "\
        [Name: Favorite Person]\n\
        [Num: 123]\n\
        ";

        let contact = Contact::new(Note::from_str(text).unwrap()).unwrap();

        let cond_true1 = Condition::Filter(
            "Num".into(),
            FilterOp::LessEq,
            "200".into()
        );
        let cond_true2 = Condition::Filter(
            "Num".into(),
            FilterOp::LessEq,
            "123".into()
        );

        let cond_false1 = Condition::Filter(
            "Name".into(),
            FilterOp::LessEq,
            "Other".into()
        );
        let cond_false2 = Condition::Filter(
            "Num".into(),
            FilterOp::LessEq,
            "20".into()
        );
        let cond_false3 = Condition::Filter(
            "Stuff".into(),
            FilterOp::LessEq,
            "5".into()
        );

        assert!(contact.matches(&cond_true1));
        assert!(contact.matches(&cond_true2));
        assert!(! contact.matches(&cond_false1));
        assert!(! contact.matches(&cond_false2));
        assert!(! contact.matches(&cond_false3));
    }

    #[test]
    fn filter_greater_than() {
        let text = "\
        [Name: Favorite Person]\n\
        [Num: 123]\n\
        ";

        let contact = Contact::new(Note::from_str(text).unwrap()).unwrap();

        let cond_true = Condition::Filter(
            "Num".into(),
            FilterOp::GreaterThan,
            "20".into()
        );

        let cond_false1 = Condition::Filter(
            "Name".into(),
            FilterOp::GreaterThan,
            "Other".into()
        );
        let cond_false2 = Condition::Filter(
            "Num".into(),
            FilterOp::GreaterThan,
            "123".into()
        );
        let cond_false3 = Condition::Filter(
            "Num".into(),
            FilterOp::GreaterThan,
            "200".into()
        );
        let cond_false4 = Condition::Filter(
            "Stuff".into(),
            FilterOp::GreaterThan,
            "5".into()
        );

        assert!(contact.matches(&cond_true));
        assert!(! contact.matches(&cond_false1));
        assert!(! contact.matches(&cond_false2));
        assert!(! contact.matches(&cond_false3));
        assert!(! contact.matches(&cond_false4));
    }

    #[test]
    fn filter_greater_equal() {
        let text = "\
        [Name: Favorite Person]\n\
        [Num: 123]\n\
        ";

        let contact = Contact::new(Note::from_str(text).unwrap()).unwrap();

        let cond_true1 = Condition::Filter(
            "Num".into(),
            FilterOp::GreaterEq,
            "20".into()
        );
        let cond_true2 = Condition::Filter(
            "Num".into(),
            FilterOp::GreaterEq,
            "123".into()
        );

        let cond_false1 = Condition::Filter(
            "Name".into(),
            FilterOp::GreaterEq,
            "Other".into()
        );
        let cond_false2 = Condition::Filter(
            "Num".into(),
            FilterOp::GreaterEq,
            "200".into()
        );
        let cond_false3 = Condition::Filter(
            "Stuff".into(),
            FilterOp::GreaterEq,
            "5".into()
        );

        assert!(contact.matches(&cond_true1));
        assert!(contact.matches(&cond_true2));
        assert!(! contact.matches(&cond_false1));
        assert!(! contact.matches(&cond_false2));
        assert!(! contact.matches(&cond_false3));
    }

    #[test]
    fn filter_not_equal() {
        let text = "\
        [Name: Favorite Person]\n\
        [Num: 123]\n\
        ";

        let contact = Contact::new(Note::from_str(text).unwrap()).unwrap();

        let cond_true1 = Condition::Filter(
            "Name".into(),
            FilterOp::Not,
            "Other Person".into()
        );
        let cond_true2 = Condition::Filter(
            "Num".into(),
            FilterOp::Not,
            "12".into()
        );
        let cond_true3 = Condition::Filter(
            "Stuff".into(),
            FilterOp::Not,
            "a".into()
        );

        let cond_false1 = Condition::Filter(
            "Name".into(),
            FilterOp::Not,
            "Favorite Person".into()
        );
        let cond_false2 = Condition::Filter(
            "Num".into(),
            FilterOp::Not,
            "123".into()
        );

        assert!(contact.matches(&cond_true1));
        assert!(contact.matches(&cond_true2));
        assert!(contact.matches(&cond_true3));
        assert!(! contact.matches(&cond_false1));
        assert!(! contact.matches(&cond_false2));
    }

    #[test]
    fn filter_cond_and_cond() {
        let text = "\
        [Name: Favorite Person]\n\
        [Num: 123]\n\
        ";

        let contact = Contact::new(Note::from_str(text).unwrap()).unwrap();

        let cond_true_a = Condition::Filter(
            "Name".into(),
            FilterOp::EqualTo,
            "Favorite Person".into()
        );
        let cond_true_b = Condition::Filter(
            "Num".into(),
            FilterOp::EqualTo,
            "123".into()
        );

        let cond_false_a = Condition::Filter(
            "Num".into(),
            FilterOp::LessThan,
            "10".into()
        );
        let cond_false_b = Condition::Filter(
            "Name".into(),
            FilterOp::EqualTo,
            "Other Person".into()
        );

        let cond_true = Condition::And(
            Box::new((cond_true_a.clone(), cond_true_b.clone())));
        let cond_false1 = Condition::And(
            Box::new((cond_true_a, cond_false_a.clone())));
        let cond_false2 = Condition::And(
            Box::new((cond_false_a, cond_false_b)));

        assert!(contact.matches(&cond_true));
        assert!(! contact.matches(&cond_false1));
        assert!(! contact.matches(&cond_false2));
    }

    #[test]
    fn filter_cond_or_cond() {
        let text = "\
        [Name: Favorite Person]\n\
        [Num: 123]\n\
        ";

        let contact = Contact::new(Note::from_str(text).unwrap()).unwrap();

        let cond_true_a = Condition::Filter(
            "Name".into(),
            FilterOp::EqualTo,
            "Favorite Person".into()
        );
        let cond_true_b = Condition::Filter(
            "Num".into(),
            FilterOp::EqualTo,
            "123".into()
        );

        let cond_false_a = Condition::Filter(
            "Num".into(),
            FilterOp::LessThan,
            "10".into()
        );
        let cond_false_b = Condition::Filter(
            "Name".into(),
            FilterOp::EqualTo,
            "Other Person".into()
        );

        let cond_true1 = Condition::Or(
            Box::new((cond_true_a.clone(), cond_true_b.clone())));
        let cond_true2 = Condition::Or(
            Box::new((cond_true_a, cond_false_b.clone())));
        let cond_true3 = Condition::Or(
            Box::new((cond_false_a.clone(), cond_true_b)));
        let cond_false = Condition::Or(Box::new((cond_false_a, cond_false_b)));

        assert!(contact.matches(&cond_true1));
        assert!(contact.matches(&cond_true2));
        assert!(contact.matches(&cond_true3));
        assert!(! contact.matches(&cond_false));
    }

    #[test]
    fn filter_in_group() {
        let text = "\
        [Name: Favorite Person]\n\
        [Num: 123]\n\
        \n\
        @Employer\n\
        [Name: Some Company]\n\
        ";

        let contact = Contact::new(Note::from_str(text).unwrap()).unwrap();

        let cond_true = Condition::Filter(
            "Employer:Name".into(),
            FilterOp::EqualTo,
            "Some Company".into()
        );
        let cond_false = Condition::Filter(
            "Employer:Name".into(),
            FilterOp::EqualTo,
            "Favorite Person".into()
        );

        assert!(contact.matches(&cond_true));
        assert!(! contact.matches(&cond_false));
    }
}
