# Convert all markdown documents in the current directory to manpages and/or
# HTML files.
#
# Requires pandoc.

SRC = $(wildcard *.md)
DESTMAN=$(SRC:.1.md=.1) $(SRC:.5.md=.5) $(SRC:.7.md=.7)
DESTHTML=$(SRC:.1.md=.1.html) $(SRC:.5.md=.5.html) $(SRC:.7.md=.7.html)

.PHONY: clean

all: man html

man: $(SRC) $(DESTMAN)

html: $(SRC) $(DESTHTML)

%.1: %.1.md
	pandoc -s -t man -o $@ $<

%.5: %.5.md
	pandoc -s -t man -o $@ $<

%.7: %.7.md
	pandoc -s -t man -o $@ $<

%.1.html: %.1.md
	pandoc -s -t html -o $@ $<

%.5.html: %.5.md
	pandoc -s -t html -o $@ $<

%.7.html: %.7.md
	pandoc -s -t html -o $@ $<

clean:
	rm *.1
	rm *.5
	rm *.7
	rm *.html
