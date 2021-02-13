% UPIM-CONF(5) upim configuration manual 0.1.0-prerelease
% Ryan Frame (code@ryanjframe.com)
% 2021 February

# NAME

upim-conf - uPIM configuration file(s)


# DESCRIPTION

uPIM is a collection of libraries and applications to form the building blocks
for a personal information management system.


## INI File Format

uPIM uses a simple INI file format:

- groups are created by naming them within brackets.
- an equal sign ('=') is used for variable assignment.
- variables can contain a '=' character.
- leading and trailing whitespace is ignored.
- whitespace surrounding group names, variables, and values are removed.
- whitespace within group names, variable names, and values is allowed.
- a semicolon (';') at the beginning of a line denotes a comment.
- if a variable is set multiple times in a file, the last one read is kept.


## Configuration File Locations

Configurations are read from the following paths, in the following order:

1. `/etc/upim/upim.conf`
2. `$XDG_CONFIG_HOME/upim/upim.conf` (or `$HOME/.config/upim/upim.conf` if
   $XDG_CONFIG_HOME is not defined)
3. `(current working directory)/.upim.conf`

Values set in later files override the earlier values, so the priority is in the
reverse order of the list above.

Applications built upon uPIM may place their own configuration files within a
`upim` configuration directory.


# Global uPIM Configuration Variables

Some applications may override variables set from their own configuration files.


## Default Group

**template_folder** (optional)
: An absolute path to the directory that contains Note templates.

**collection_base** (optional)
: The (absolute) base path for relative paths in the Collections group. If not
  specified, all collection paths must be absolute.


## Collections Group

The collections group has not predefined variables. Collections are defined by
defining a variable of the collection's name with a value of the absolute path
to the directory containing notes within that collection.


# Standard uPIM Application Configuration Files

## upim-edit

upim-edit searches for the following files and uses the first one it finds:

1. /etc/upim/upim-edit.conf
2. $XDG_CONFIG_HOME/upim/upim-edit.conf (or $HOME/.config/upim/upim-edit.conf if
   $XDG_CONFIG_HOME is unset)

You can specify an alternative file via the `-C` option.


### Default Group

**editor**
: Specify the text editor to use. Required only if $EDITOR is not set or if the
  $EDITOR requires an **editor_arg** that upim-edit does not recognize.

**editor_arg**
: A command-line argument to tell the editor to run in the background, if
  required.

**template_folder** (optional)
: The global **template_folder** may be overridden.

**collection_base** (optional)
: The global **collection_base** may be overridden.


### Collections Group

The upim-edit configuration may append to or override the global collections.
This is likely of use only to people using multiple configurations for different
purposes (project-specific configurations, etc.).


# FILES

/etc/upim/upim.conf

$XDG_CONFIG_HOME/upim/upim.conf

$HOME/.config/upim/upim.conf


# EXAMPLES

```ini
[DEFAULT]

template_folder = /home/me/upim-templates

[Collections]
Contacts = /home/me/Contacts
Documents = /home/me/Documents
Recipes = /home/me/Documents/Recipes
KB = /home/me/notes
```

# SEE ALSO

**upim**(7)

