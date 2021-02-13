% UPIM(7) upim manual 0.1.0-prerelease
% Ryan Frame (code@ryanjframe.com)
% 2021 February

# NAME

upim - Universal Personal Information Manager


# DESCRIPTION

uPIM is a collection of libraries and applications to form the building blocks
for a personal information management system.

uPIM provides the base framework to create a personal information system; it
does not provide a full PIM system out of the box. You must think about the
information you want to manage and the process you want to use, then you can use
uPIM to create it.

The uPIM tools and libraries are organized around a few simple ideas:

* Text is universal.
* You control your information -- not uPIM.
* Efficiency is a red herring -- efficacy is the real goal.
* A collection of tools that perform a single task and interoperate via shared
  conventions will be more flexible and reliable than a monolithic system or
  central data store.


# CONFIGURATION

The global configuration is read from the following INI files if present, in
order:

1. /etc/upim/upim.conf
2. $XDG_CONFIG_HOME/upim/upim.conf (or $HOME/.config/upim/upim.conf if
   $XDG_CONFIG_HOME is unset).
3. (current working directory)/.upim.conf

Values set in later files override the earlier values, so the priority is in the
reverse order of the list above.

Applications may read from the global configuration, and may also place their
own configurations in the same directories.

See **upim.conf**(5) for configuration options.


# SEE ALSO

**upim.conf**(5), **upim-edit**(1)


# CAVEATS


# SECURITY CONSIDERATIONS

