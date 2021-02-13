#!/bin/sh

# Use pandoc to convert the given markdown document to a manpage and view it.

pandoc $1 -V hyphenate=false -s -t man | /usr/bin/man -l -
