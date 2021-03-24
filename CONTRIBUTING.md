# Contributing to uPIM

This is mostly a set of guidelines, not fixed rules. They are also changeable --
if you have questions or think something can be improved please submit an
issue/PR or start a discussion.


## Table of Contents

* [Quick Reference to Websites](#quick-reference)
* [How Can I Contribute?](#how-can-i-contribute)
* [What Do I Need for Development?](#what-do-i-need-for-development)
* [Style Guidelines](#style-guidelines)
    * [Commit and PR Style](#commit-and-pr-style)
    * [Code Style](#code-style)


## Quick Reference

* Source code repositories:
    * [https://git.sr.ht/~rjframe/upim](https://git.sr.ht/~rjframe/upim/)
    * [https://github.com/rjframe/upim](https://github.com/rjframe/upim/)
* Issue tracker:
  [https://todo.sr.ht/~rjframe/upim](https://todo.sr.ht/~rjframe/upim/)
* Wiki: [https://man.sr.ht/~rjframe/upim](https://man.sr.ht/~rjframe/upim/)
* Mailing Lists:
    * Announcements:
[https://lists.sr.ht/~rjframe/upim-announce](https://lists.sr.ht/~rjframe/upim-announce)
    * Discussions:
[https://lists.sr.ht/~rjframe/upim-discuss](https://lists.sr.ht/~rjframe/upim-discuss)
    * Development:
[https://lists.sr.ht/~rjframe/upim-devel](https://lists.sr.ht/~rjframe/upim-devel)


## How Can I Contribute?

* Testing: test the applications to ensure they work as you'd expect. Try to
  break things and [report problems](https://todo.sr.ht/~rjframe/upim/).
* [Create tickets](https://todo.sr.ht/~rjframe/upim) for discovered bugs and
  ideas for improvement.
* [Write documentation](https://man.sr.ht/~rjframe/upim/): from tutorials and
  reference material to answering questions other people have.
* Write code for uPIM.
* Share the progams and scripts you create using uPIM.


## What Do I Need for Development?

You will need the nightly version of the
[Rust compiler](https://www.rust-lang.org/tools/install) and cargo build tool.
If you wish to build the manpages you will need
[scdoc](https://sr.ht/~sircmpwn/scdoc/).

You can build the libraries and programs by running `cargo build` or run tests
via `cargo test`. To build the manpages run `make` from the `docs/manpages`
directory. You can generate and view API reference documentation by running
`cargo doc --open`.


## Style Guidelines

## Commit and PR Style

The first line of a commit message should summarize the purpose of the commit.
It should be a full sentence but end without a period. The subject must be no
more than 72 characters, preferably no more than 50.

If the commit addresses a specific crate or module, prefix the commit message
with that crate name; for libraries, omit the "upim-" prefix but keep it for
applications (use "upim-edit" and "upim-find" but "note" and "core".

Write the subject in imperative style (like you're telling someone what to do);
use "Add xyz" instead of "Added xyz", "Fix" instead of "Fixed", etc.

Example commit subjects:

```
upim-edit: Allow overriding global template_folder
upim-edit: Offer to re-open file if validation fails
note: Add method to read only a note header
core: Add uniq iterator
```

If relevant, later paragraphs should provide context and explain anything that
may not be apparent; for example, if you made a design decision that may not be
obvious, why did you choose that over an alternative?

Answer the question "why?"; we can see "what" from the code itself. Use "Fix
typo in schedule documentation" rather than "Change schedull to schedule".

Text should be wrapped at 72 characters.

If a commit references, is related to, or fixes an issue, list it at the end.

A full commit message might look something like this:

```
Add a widget to the box.

The box was looking empty with nothing inside it.

We could also have used a gadget, but widgets are shiny, and I like
shiny things.

This does mean we will no longer be able to fit some things inside the
box:

* contrivances will be too big
* devices might break nearby widgets
* gimmicks would no longer be relevant

Resolves: #4
```

It's best to keep commits small when possible, doing only one thing.

PRs that are only cosmetic (style) fixes will typically not be accepted since
this messes up `git blame`. Style-only commits in the code you're working with
while doing something else are fine, but the style fixes should be in a separate
commit from functional changes.


### Code Style

* Use four spaces for indentation.
* Use a hard 80 column line width.
* Write code with understandability and future maintainability in mind.
* Write tests whenever practical; exercise error conditions and edge cases as
  well as the happy path.
* Document all public declarations. Also document non-trivial private
  declarations.
* Follow the typical Rust naming conventions.
* If an import is used in one or very few places in a module, prefer a local
  import to a global one (import inside the function rather than the top of the
  file).
* In general, try to conform to the style of the code in which you're working.


#### Braces

Place opening braces on the same line as the function declaration/if
expression/etc. unless doing so would break the 80 column rule.


#### Function Definitions

If a function's return value would cross 80 columns, place the `->` on the next
line at column 1. `where` clauses should be indented four spaces, and the
opening brace should be placed on the next line on column 1.

Examples:

```rust
fn my_function(a: u32) -> u32 {
    // Stuff
}

fn my_function_with_a_long_name(apple: u32, banana: &str, carrot: f32)
-> FoodFromIngredients {
    // Stuff
}

fn another_function<T>(food: &T) -> u32
    where T: SomeTrait,
{
    // Stuff
}

fn my_other_function_with_a_long_name<T>(apple: T, banana: &str, carrot: f32)
-> FoodFromIngredients
    where T: SomeTrait,
{
    // Stuff
}
```
