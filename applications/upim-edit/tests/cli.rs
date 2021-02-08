use std::{
    fs::{File, remove_file},
    io::Write,
    path::PathBuf,
    process::{Command, Output},
    env,
    str,
};

use rand::{
    distributions::Alphanumeric,
    Rng,
    thread_rng,
};

use upim_note::Note;


// TODO: Find a better way to do this.
const UPIM_EDIT: &str = "../../target/debug/upim-edit";

/// Retrieve a path to a non-existent file in a temporary directory.
fn temp_file() -> PathBuf {
    let mut rng = thread_rng();
    let path = env::temp_dir();

    let file = loop {
        let name: String = (&mut rng).sample_iter(Alphanumeric)
            .take(4)
            .map(char::from)
            .collect();

        let mut file = path.clone();

        file.push(name);
        file.set_extension("txt");

        if ! file.exists() { break file; }
    };

    println!("* temporary file path: {:?}", file);
    file
}

/// Create a temporary file with the provided data.
fn temp_file_with(content: &str) -> (PathBuf, File) {
    let path = temp_file();
    let mut file = File::create(&path).unwrap();

    file.write_all(content.as_bytes()).unwrap();

    (path, file)
}

fn exec(command: &str, args: &[&str]) -> Output {
    Command::new(command)
        .args(args)
        .output()
        .expect("Failed to execute process")
}


#[test]
fn add_tags_to_file() {
    let (path, file) = temp_file_with("\
    @tag1 @tag2\n\
    \n\
    Some content.\n\
    ");

    exec(UPIM_EDIT, &["--add-tags", "@tag3", "@tag4", path.to_str().unwrap()]);

    let note = Note::read_from_file(path.to_str().unwrap()).unwrap();

    assert!(note.contains_tag("@tag1"));
    assert!(note.contains_tag("@tag2"));
    assert!(note.contains_tag("@tag3"));
    assert!(note.contains_tag("@tag4"));

    remove_file(path);
}

#[test]
fn remove_tags() {
    let (path, file) = temp_file_with("\
    @tag1 @tag2 @tag3 @tag4\n\
    \n\
    Some content.\n\
    ");

    exec(UPIM_EDIT,
        &["--remove-tags", "@tag3", "@tag4", path.to_str().unwrap()]
    );

    let note = Note::read_from_file(path.to_str().unwrap()).unwrap();

    assert!(note.contains_tag("@tag1"));
    assert!(note.contains_tag("@tag2"));
    assert!(! note.contains_tag("@tag3"));
    assert!(! note.contains_tag("@tag4"));

    remove_file(path);
}

#[test]
fn list_tags() {
    let (path, file) = temp_file_with("\
    @tag1 @tag2\n\
    \n\
    Some content.\n\
    ");

    let output = exec(UPIM_EDIT, &["--tags", path.to_str().unwrap()]);
    let output = str::from_utf8(&output.stdout).unwrap();

    assert_eq!(output, "@tag1\n@tag2\n");

    remove_file(path);
}

#[test]
fn add_attribute_to_file() {
    let (path, file) = temp_file_with("\
    @tag\n\
    [key1: value1]\n\
    [key2: value2]\n\
    \n\
    Some content.\n\
    ");

    exec(UPIM_EDIT, &["--add-attr", "key3", "value3", path.to_str().unwrap()]);

    let note = Note::read_from_file(path.to_str().unwrap()).unwrap();

    assert_eq!(note["key1"], "value1");
    assert_eq!(note["key2"], "value2");
    assert_eq!(note["key3"], "value3");

    remove_file(path);
}

#[test]
fn remove_attribute() {
    let (path, file) = temp_file_with("\
    @tag\n\
    [key1: value1]\n\
    [key2: value2]\n\
    [key3: value3]\n\
    \n\
    Some content.\n\
    ");

    exec(UPIM_EDIT, &["--remove-attr", "key2", path.to_str().unwrap()]);

    let note = Note::read_from_file(path.to_str().unwrap()).unwrap();

    assert_eq!(note["key1"], "value1");
    assert_eq!(note["key3"], "value3");
    assert!(! note.contains_attribute("key2"));

    remove_file(path);
}

#[test]
fn list_attributes() {
    let (path, file) = temp_file_with("\
    @tag\n\
    [key1: value1]\n\
    [key2: value2]\n\
    \n\
    Some content.\n\
    ");

    let output = exec(UPIM_EDIT, &["--attributes", path.to_str().unwrap()]);
    let output = str::from_utf8(&output.stdout).unwrap();

    // We don't know the order in which attributes will be printed to stdout.
    assert!(output.contains("key1:value1"));
    assert!(output.contains("key2:value2"));
    assert!(! output.contains("@tag"));
    assert!(! output.contains("Some content"));

    remove_file(path);
}
