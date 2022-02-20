CRATES= cpclib-sna \
		cpclib-tokens\
		cpclib-disc \
		cpclib-basic\
		cpclib-xfer\
		cpclib-xfertool
		 cpclib-image \
		cpclib-asm\
		cpclib-z80emu\
		cpclib-macros \
		cpclib \
		basm \
		bdasm \
		imgconverter
publish:
	for project in $(CRATES) ; \
	do cd $$project ; \
	   cargo +nightly publish || exit -1; \
	   sleep 10 ; \
	   cd ..; \
	done


fmt:
	cargo fmt