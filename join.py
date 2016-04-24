
# Utility script to join part files
# usage - part_file name, parts range (exclusive)

import sys
with open( str(sys.argv[1]),"ab") as out_file:
	for i in range(0,int(sys.argv[2])):
		with open("{}{}".format(str(sys.argv[1]),i),'rb') as f:
			out_file.write(f.read())
