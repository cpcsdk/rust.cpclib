
const CODE_BASIC = 0;
const CODE_BASM = 1;
const EMU_URL = "emu/tiny8bit/cpc-ui.html";

var urlToRevoke = null;

import('../pkg')
  .then((cpcasm) => {

	init();

	function init() {
		cpcasm.init_panic_hook();

		window.document.getElementById("action_run")
			.onclick = requestRun;

		window.document.getElementById("action_download")
			.onclick = requestDownload;

	}


	/**
	 * Assemble the source and launch the program in case of success
	 */
	function requestRun(event) {
		// Build the project
		var sna = build();
		if (null != sna) { launch_sna(getProjectName()+".sna", sna)}
	}

	function requestDownload(event) {
		// Build the project
		var sna = build();
		if (null != sna) { sna.download(getProjectName()+".sna")}
	}


	function launch_sna(fname, sna) {
		cpc_inject_snapshot(fname, sna)
	}

	// try to load from drop
	// Does not work
	function launch_drop(sna) {

		var emu = window.document.getElementById("emu")
		.contentDocument 
	/*	.getElementById("canvas")*/;

		console.log(emu);
		emu
			.handleDrop({
				dataTransfer: {files: [sna]},
				preventDefault: function(){}
			});

	}

	// try to load from base 64
	// Does not work

	function launch_base64(sna) {
		let url = sna.toString('base64')
		console.log(url);

		window.document.getElementById("emu")
			.src = EMU_URL + "?file=" + url;

	}

	// launch by ending a blob
	// Does not work
	function launch_blob(sna) {
		if (null!=urlToRevoke) {
			URL.revokeObjectURL(urlToRevoke);
			urlToRevoke = null;
		}

		var blob = new Blob([sna.bytes], {type: "application/octet-stream"});
		var url = URL.createObjectURL(blob);

		window.document.getElementById("emu")
			.src = EMU_URL + "?file=" + encodeURIComponent(url);

		urlToRevoke = url;
	}

	function build() {
		var source = getSourceCode();
		var sna = null;
		
		try {
			switch (getCodeKind()) {
				case CODE_BASM:
					var fname = getProjectName() + ".asm";
					var config = cpcasm.asm_create_parser_config(fname);
					sna = cpcasm.asm_assemble_snapshot(source, config);
					break;

				case CODE_BASIC:
					sna = cpcasm.basic_snapshot(basic);
					break;
			}

			return sna;
		} catch (error) {
			show_error(error);
			return null;
		}

	}

	function show_error(e) {
		window.document.getElementById("error")
			.innerText = e;
	}

	function getSourceCode() {
		return window.document.getElementById("source_code")
						.innerText;
	}

	/**
	 * Return the kind of source code manipulated.
	 * Should be retreived from the interface
	 */
	function getCodeKind() {
		return CODE_BASM;
	}

	/**
	 * Return the project name.
	 * Should be retreived from the interface.
	 * Must not contain space
	 */
	function getProjectName() {
		return "test";
	}

	function test () {
		console.log(cpcasm);

		// mandatory init
	//	();

		// test parse error
		var config = cpcasm.asm_create_parser_config("test");
		var source = "ld hl, 1234  push hl";
		try {
			var result = cpcasm.asm_parse_source(source, config);
			console.error("ERR. Parsing successfull", result);
		} catch(error) {
			console.info("OK. Expected error\n",error.msg);
		}

		// test parse ok
		config = cpcasm.asm_create_parser_config("test");
		source = "ld hl, 1234 : push hl";
		try {
			result = cpcasm.asm_parse_source(source, config);
			console.info("Ok. Parse successful", result);
		} catch(error) {
			console.error("ERR. Unexpected error\n",error.msg);
		}



		// test assemble error
		var config = cpcasm.asm_create_parser_config("test");
		var source = "ld hl, 1234  push hl";
		try {
			var result = cpcasm.asm_assemble_snapshot(source, config);
			console.error("ERR. assembling successfull", result);
		} catch(error) {
			console.info("Ok. Expected error\n",error.msg);
		}




		// test assemble ok
		config = cpcasm.asm_create_parser_config("test");
		source = "ld hl, 1234 : push hl";
		try {
			result = cpcasm.asm_assemble_snapshot(source, config);
			console.info("Ok. Parse successful", result);
		} catch(error) {
			console.error("ERR. Unexpected error\n",error.msg);
		}


		// test download snapshot
		config = cpcasm.asm_create_parser_config("test");
		source = " org 0x4000: di : jp $ ";
		try {
			sna = cpcasm.asm_assemble_snapshot(source, config);
			console.info("Ok. Parse successful", sna);

			// force a download of the sna to test on a real emulator
			var blob = new Blob([sna.bytes], {type: "application/octet-stream"});
			let link = document.createElement('a');
			link.download = 'test.sna';
			link.href = URL.createObjectURL(blob);
			link.click();
			URL.revokeObjectURL(link.href);

		} catch(error) {
			console.error("ERR. Unexpected error\n",error.msg);
		}

		// Basic
		try {
			basic = "10 PRINT \"HELLO\":20 PRINT \"WORLD\"";
			sna = cpcasm.basic_parse_program(basic).sna();
			sna.download("basic_rust.sna");

			sna = cpcasm.basic_parse_program(basic).sna();

			var blob = new Blob([sna.bytes], {type: "application/octet-stream"});
			let link = document.createElement('a');
			link.download = 'basic_js.sna';
			link.href = URL.createObjectURL(blob);
			link.click();
			URL.revokeObjectURL(link.href);
		} catch(error) {
			console.error("ERR. Unexpected error\n",error);
		}
	}
  })