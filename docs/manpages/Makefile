# Convert all markdown documents in the current directory to manpages and/or
# HTML files.
#
# Requires pandoc.

SRC = $(wildcard *.scdoc)
DESTMAN=$(SRC:.1.scdoc=.1) $(SRC:.5.scdoc=.5) $(SRC:.7.scdoc=.7)
#DESTHTML=$(SRC:.1.md=.1.html) $(SRC:.5.md=.5.html) $(SRC:.7.md=.7.html)

.PHONY: clean

all: man

man: clean $(SRC) $(DESTMAN)

#html: $(SRC) $(DESTHTML)

%.1: %.1.scdoc
	scdoc < $< > $@

%.5: %.5.scdoc
	scdoc < $< > $@

%.7: %.7.scdoc
	scdoc < $< > $@

#%.1.html: %.1.scdoc
#	pandoc -s -t html -o $@ $<

#%.5.html: %.5.scdoc
#	pandoc -s -t html -o $@ $<

#%.7.html: %.7.scdoc
#	pandoc -s -t html -o $@ $<

clean:
	rm -f *.1
	rm -f *.5
	rm -f *.7
