#!/bin/bash


OUTPUT="/tmp/basm1.o"
OUTPUT2="/tmp/basm2.o"
ERR="/tmp/basm_err.o"

#export RUSTFLAGS=-Awarnings

# ensure assmebler exists
cargo build --bin basm --features basm || exit -1

BASM="$(git rev-parse --show-toplevel)/target/debug/basm"

declare -a wrong_files

function build(){
	"${BASM}" -i "$1" -o "$OUTPUT" >/dev/null 2>"$ERR"
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
	current_error=1

}

ok=0
err=0

for fname in good_*.asm
do
	# skip tests when argument provided
	if test $# -eq 1 && [[ ! "$fname" == *"$1"* ]]
	then
		continue
	fi

	current_error=0

	if build "$fname"
	then		

		# compare to a binary file
		binary="${fname%.*}.bin"
		if test -e "$binary"
		then
			# binary comparison
			if ! diff "$OUTPUT" "$binary" > /dev/null 2>/dev/null
			then
				err_result "$fname" content

				hexdiff "$OUTPUT" "$binary"
			fi
		fi

		# vompare to a similar asm file
		equiv="${fname%.*}.equiv"
		if test -e "$equiv"
		then
			mv "$OUTPUT" "$OUTPUT2"
			if ! build "$equiv"
			then
				echo "** ERROR IN TEST PROCEDURE EQUIV FILE NOT ASSEMBLED **" >&2
				err_result "$equiv" content
				exit -1
			fi

			# check if file is assembled as its equivalent
			if ! diff "$OUTPUT2" "$OUTPUT" > /dev/null 2>/dev/null
			then
				err_result "$fname" equiv
			fi
		fi

		# no failure
		if test $current_error -eq 0
		then
			ok_result  "$fname"
		fi
	else
		# assemble error
		err_result "$fname" assemble
	fi
done

for fname in bad_*.asm
do
	# skip tests when argument provided
	if test $# -eq 1 && [[ ! "$fname" == *"$1"* ]]
	then
		continue
	fi

	if ! build "$fname"
	then
		ok_result  "$fname"
	else
		err_result "$fname" assemble
	fi
done

echo
echo "$ok successes"
if ! test $err -eq 0
then
	echo "$err failures:"  ${wrong_files[@]}
fi

test $err -eq 0