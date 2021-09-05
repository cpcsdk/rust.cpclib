#!/bin/bash


function build(){
	../../../target/debug/basm -i "$1" >/dev/null 2>/dev/null
}

function ok_result() {
	ok=$((ok+1))
	echo "[OK]  $1"
}

function err_result() {
	err=$((err+1))
	echo "[ERR] $1"
}

ok=0
err=0

for fname in good_*
do
	if build "$fname"
	then
		ok_result  "$fname"
	else
		err_result "$fname"
	fi
done

for fname in bad_*
do
	if ! build "$fname"
	then
		ok_result  "$fname"
	else
		err_result "$fname"
	fi
done

echo
echo "$ok successes"
echo "$err failures"

test $err -eq 0