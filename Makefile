CRATES= cpclib-sna \
		cpclib-tokens\
		cpclib-disc\
		cpclib-basic\
		cpclib-xfer\
		cpclib-tool\
		cpclib-asm\
		cpclib-z80emu\
		cpclib
publish:
	for project in $(CRATES) ; \
	do cd $$project ; \
	   cargo publish || exit -1; \
	   cd ..; \
	done