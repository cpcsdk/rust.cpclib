
import('../pkg')
  .then((cpcasm) => {

	console.log(cpcasm);

	// mandatory init
//	init_panic_hook();

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
		console.error("ERR. Unexpected error\n",error.msg);
	}
  }) 