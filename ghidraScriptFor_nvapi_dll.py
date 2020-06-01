#TODO write a description for this script
#@author 
#@category Examples.Python
#@keybinding Ctrl-MINUS
#@menupath Tools.Misc.getParamTypes
#@toolbar 
from __future__ import print_function

import re
import pprint
import json
import struct
import sys
import os.path as path
from os.path import expanduser
import ghidra.app.decompiler.DecompInterface as DecompInterface
import ghidra.app.decompiler.ClangStatement as ClangStatement

start = 0x10395010
end = 0x10398af0
# currentLocation.getAddress().getOffset()
print('\n\n\n\n\n\n')
print("Getting info from functions. Looking at {} to {}".format(hex(start), hex(end)))

codes = {
	"0x0150e828": "Initialize",
	"0xd22bdd7e": "Unload",
	"0x891fa0ae": "SetCoolerLevels",
	"0xda141340": "GetCoolerSettings",
	"0x189a1fdf": "GetUsages",
	"0xfb85b01e": "ClientFanCoolersGetInfo",
	"0x35aed5e8": "ClientFanCoolersGetStatus",
	"0x814b209f": "ClientFanCoolersGetControl",
	"0xa58971a5": "ClientFanCoolersSetControl",
	"0xe5ac921f": "ApiSupported",
	}
# ls = list(locals())
# for l in ls:
# 	if l.startswith('get'):
# 		print(l)
data = []
def run():
	decomp = DecompInterface()
	decomp.openProgram(currentProgram)
	regex = re.compile(r"[^\n(]+\(([^\n]*)\);")
	c = 0
	for i in (range(start, end + 8, 8)):
		c += 1
		fun = i
		code = int(bin(getInt(toAddr(i+4))), 2)
		# stupid conversion with struct because there is no getUint -_-
		code = hex(struct.unpack('I', struct.pack('i', code))[0])[:-1]
# 		if not codes.get(code):
# 			continue
		addr = toAddr(getInt(toAddr(fun)))
		fdata = {
			"address": "0x{}".format(str(addr)),
			"query_code": code
		}
		data.append(fdata)
		print("XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX")
# 		print("Function {}".format(c))
		print("Query Code: {}".format(code))
		print("Function Address: {} ".format(fdata["address"]))

# 		print("Function Name: {}".format(codes[code]))
		fn = getFunctionAt(addr)
		if fn is None:
			fdata["name"] = "No Function here"
# 			print("No Function here!")
			continue
	# 	print(fn) == print(fn.getName())
# 		print("Function Name: {}".format(fn.getName()))
		fdata["name"] = fn.getName()
		entry = fn.getEntryPoint()
	#   print(entry.toString()) == print(addr)
		params = fn.getParameters()

		monitor.setMessage("Decompiling Function {}".format(c))
		decres = decomp.decompileFunction(fn, 1000, monitor)
# 		print(decres.getCCodeMarkup())
		signature = decres.getDecompiledFunction().getSignature()
# 		print(signature)
		fdata["signature"] = signature
		matches = regex.match(signature)
		params = matches.groups()[0].split(',')
		fdata["parameters"] = params
		pointers = [x.count('*') for x in params]
# 		print(pointers)
		print(json.dumps(fdata, indent=2))
	sys.stdout = open(expanduser('~') + '\\functions.txt', "w")
	print(json.dumps(data, indent=2))
run()