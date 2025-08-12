ld hl,plop

module grouik
plop: ifused plop : assert 0==1 : endif
endmodule

ifnused plop : assert 0==1 : endif
plop nop