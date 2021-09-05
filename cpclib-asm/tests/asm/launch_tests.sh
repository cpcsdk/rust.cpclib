#!/bin/bash


OUTPUT="/tmp/basm.o"
ERR="/tmp/basm_err.o"

declare -a wrong_files

function build(){
	../../../target/debug/basm -i "$1" -o "$OUTPUT" >/dev/null 2>"$ERR"
}

function ok_result() {
	ok=$((ok+1))
	echo "[OK]  $1"
}

function err_result() {
	err=$((err+1))
	echo "[ERR] $1 ($2)"
	cat "$ERR"
	wrong_files=( "${wrong_files[@]}" $1)
}

ok=0
err=0

for fname in good_*.asm
do
	if build "$fname"
	then
		binary="${fname%.*}.bin"
		# assemble ok
		if test -e "$binary"
		then
			# binary comparison
			if diff "$OUTPUT" "$binary" > /dev/null 2>/dev/null
			then
				ok_result  "$fname"
			else
				err_result "$fname" content
			fi
		else
			# no need to compare binary
			ok_result  "$fname"
		fi
	else
		# assemble error
		err_result "$fname" assemble
	fi
done

for fname in bad_*.asm
do
	if ! build "$fname"
	then
		ok_result  "$fname"
	else
		err_result "$fname" assemble
	fi
done

echo
echo "$ok successes"
echo "$err failures:"  ${wrong_files[@]}

test $err -eq 0