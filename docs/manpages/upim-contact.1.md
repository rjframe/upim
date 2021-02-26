% UPIM-CONTACT(1) upim-contact manual 0.1.0-prerelease
% Ryan Frame (code@ryanjframe.com)
% 2021 February

# NAME

upim-contact - the uPIM contact manager

# SYNOPSIS

**upim-contact** [-C *COLLECTION*] [--conf *PATH*] **new** *NAME*

**upim-contact** [-C *COLLECTION*] [--conf *PATH*] **edit** *NAME* | *PATH*

**upim-contact** [*FILTER-ALIAS*] [*ALIAS-ARGUMENTS*] [*OPTION*...]


# DESCRIPTION

uPIM is a collection of libraries and applications to form the building blocks
for a personal information management system.

upim-contact uses a flexible, human-readable format for storing contact
data to help you quickly find the person(s) you're looking for right now.

See **upim-contact**(5) for details on the contact note format.


# OPTIONS

## General Options

**-C** *COLLECTION-NAME*
: Use the specified collection rather than the default

**\-\-conf** *PATH*
: Use the specified configuration file instead of the default. The global uPIM
  configuration file is still read


## Query Options

**\-\-filter** *FILTER-STRING*
: Filter the contacts according to the given filter string. If multiple
  **\-\-filter** options are given, the intersection (logical AND) is applied

**\-\-limit** *LIMIT*
: The maximum number of contact records to output. Invalid input and numbers
  below 1 are ignored

**\-\-sort-a** *FIELD-NAME*
: sort contacts by the given field in ascending order

**\-\-sort-d** *FIELD-NAME*
: sort contacts by the given field in descending order


# Commands

**new** *NAME*
: Create a new contact with the given name in the default collection

**edit** *NAME* | *FILE*
: Edit the given file or first discovered contact with the specified name


# Search Aliases

Search aliases are commands defined in **upim-contact.conf** to provide
pre-defined filters. See **upim-conf**(5) for information on creating
search aliases.

Search aliases may have arguments; for example, the default **find** alias takes
a string to match against contacts' Name fields:

```shell
$ upim-contact find 'Favorite Person'
```

The above command with the default "find" alias definition is equivalent to:

```shell
$ upim-contact --filter 'Name,Phone,Employer:Name' WHERE Name = 'Favorite Person'
```

Filters may be provided to further limit results:

```shell
$ upim-contact find 'Favorite Person' --filter '* WHERE Phone NOT EMPTY'
```


# CONFIGURATION


# FILES


# EXAMPLES


# SEE ALSO

**upim**(7), **upim-contact**(5), **upim-conf**(5)
