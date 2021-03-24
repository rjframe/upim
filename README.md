# uPIM - Universal Personal Information Management

uPIM is a collection of libraries and applications to form the building blocks
for a personal information management system.

uPIM is currently early-stage software. I am already using the note format for
all my documents (knowledge base, recipes, contacts, etc.) but there is a lot of
work still to do before most people will find any use from it.

The official repository for uPIM is on SourceHut at
[https://git.sr.ht/~rjframe/upim](https://git.sr.ht/~rjframe/upim), with a
mirror at Github on
[https://github.com/rjframe/upim](https://github.com/rjframe/upim). Patches/pull
requests from both are accepted; all other activity takes place at sr.ht.

* Issues: [https://todo.sr.ht/~rjframe/upim](https://todo.sr.ht/~rjframe/upim/)
* Discussions:
  [https://lists.sr.ht/~rjframe/upim-discuss](https://lists.sr.ht/~rjframe/upim-discuss)
* Wiki: [https://man.sr.ht/~rjframe/upim/](https://man.sr.ht/~rjframe/upim/)


## Table of Contents

* [Introduction](#introduction)
    * [License](#license)
* [Getting Started](#getting-started)
    * [Install](#application-installation)
    * [Overview](#overview)
    * [Applications](#applications)
    * [Libraries](#libraries)
    * [Known Issues](#known-issues)
* [Contributing](#contributing)
* [Contact](#contact)
* [Related Projects](#related-projects)


## Introduction

Other attempts to create a fully-flexible PIM have tried to build monolithic
systems that could organize and interpret every conceivable kind of information,
and often discovered the project to be too ambitious, failing as the project's
scope increased faster than they could implement their designs.

uPIM is organized differently: software is the tool, not the solution. By
building a library and tool suite around a few simple ideas, we create not a
one-size-fits all solution but the tools that you can use to create your own PIM
solution and information management process.

This allows us to support an incredible degree of flexibility and
interoperability -- uPIM does not have to understand your data, and instead of
replacing your current tools, you can extend them.

* Text is universal.
* You control your information -- not uPIM.
* A collection of tools that perform a single task and interoperate via shared
  conventions will be more flexible and reliable than a monolithic system or
  central data store.


## License

All source code is licensed under the terms of the
[MPL 2.0 license](LICENSE.txt).


## Getting Started

### Application Installation

Rust nightly is required.

Application sources are in the `applications` directory. Run
`cargo build --release` to build them. Default configuration files are provided
in the `config` directory; to install them:

* **Unixy:** Place the configuration files in `$HOME/.config/uPIM/`
* **Windows:** Rename the file extensions to .ini, then place them in
  `%APPDATA%\uPIM\`.

Edit the configuration files -- at minimum, set the `template_folder` variable
in upim.conf (upim.ini) and create your collections. There are default
configuration files in the config directory that provide a good starting place.

See the [upim-conf(5)](docs/manpages/upim-conf.5.scdoc) manual in the `docs`
directory for more information about supported locations and the configurable
options.


### Libraries

Use `cargo doc` to build the library documentation locally. You can list the
full suite or specific subcrates as dependencies.

I've not yet made a release or registered uPIM on crates.io, so clone the
repository from git and add upim to your Cargo.toml as a local dependency:

```toml
[dependencies]

upim = { path = "../upim" }

# Or individual crates:
upim-core = { path = "../upim/upim-core" }
```


### Overview

Most libraries and applications utilize "Notes" -- text documents containing a
header comprised of arbitrary tags and key-value attributes, followed by the
document itself.

Tags begin with '@', then contain any non-space text. Attributes are surrounded
by '[' and ']', with the key separated by a colon. The header ends with a blank
line:

```
@tag1 @tag2
[Key: Value]
@tag3

This is the document. Multiple tags can be on a line, but only one attribute,
and attributes cannot span multiple lines.
```

Recipe example as a markdown document:

```md
@low-fat @low-carb
[Name: My Favorite Food]
[Author: Some Person]
[Prep time: 5 minutes, maybe a few days]

# My Favorite Food

This is a great meal to serve for guests.

## Ingredients

* Water
* Salt

## Instructions

Mix the water and salt together. Let dry.
```

Storing contact information with recursive notes:

```
@family @last-update:2000-01-01
[Name: Favorite Person]
[Spouse: Other Person]
[Children: A Person, B Person, C Person]

[Address: 123 Somewhere]
[Phone: 123-456]

@Employer
[Name: Some Company]
[Website: www.example.com]

And there's plenty of room for whatever prose you wish to record.
```

Using notes for something like contact management provides more flexibility than
vCard, while being easily converted to and from vCard via standard
(user-configurable) attributes.


### Applications

The uPIM applications are designed to:

* provide a simple textual interface for most basic or daily needs (editing,
  searching, etc.)
* provide a scriptable interface for process automation and customization

Applications listed below in _italics_ have not yet been created.

Name           | Description
-------------- | ----------------------------------------------------------
upim-edit      | Wraps your text editor to edit notes, manages note headers
_upim-find_    | Searches uPIM collections
_upim-contact_ | Contact manager
_upim-cal_     | Calendar integration


### Libraries

The uPIM core libraries provide the basic and common functions for uPIM
applications to interoperate. If you wish to extend uPIM, you'll probably want
to use these.

Libraries listed below in _italics_ have not yet been created.

Name        | Description
----------- | ---------------------------------------------------------------
upim-core   | Shared configuration, containers, error types, algorithms, etc.
upim-note   | Read, write, and parse notes
_upim-conv_ | Conversions between Notes and other standard file formats


### Known Issues

* Some string parsing assumes single-byte characters. No such functions are
  data-corrupting.
* Applications:
    * Documentation is unixy-centric.
    * *upim-edit*: Some invalid arguments may have incorrect error messages,
      especially if the file to edit was not provided.
* Libraries:
    * APIs are not stable yet. Once stabilized, we'll create bindings for C (and
      maybe other languages from that) so applications can be written in other
      languages.


## Contributing

Patches and pull requests are welcome. For major features or breaking changes,
please open a ticket or start a discussion first so we can discuss what you
would like to do.

See [CONTRIBUTING.md](CONTRIBUTING.md) for pointers on getting set up. If you'd
like guidance on anything feel free to ask in a discussion or ticket, or submit
a draft PR.

Some specific areas of concern I could use help with are:

* Ensuring upim-edit supports a variety of text editors
* Decisions around document/configuration file management on macOS
* Testing on Windows, macOS
* Maintainer for the Windows platform, maybe macOS


## Contact

- Email: code@ryanjframe.com
- Website: [www.ryanjframe.com](https://www.ryanjframe.com)
- diaspora*: rjframe@diasp.org


## Related Projects

* [imag](https://imag-pim.org/): command-line PIM suite
