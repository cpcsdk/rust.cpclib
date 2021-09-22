
	module module1
label1
	jp module2.label1
	endmodule



	module module2
label1 
	jp module3.module31.label1
	endmodule
	
	module module3
		module module31
label1
		jp ::label1
		endmodule
	endmodule
label1