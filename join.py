import sys

with open("Python-3.5.1.tgz","ab") as out_file:
	for i in range(0,int(sys.argv[1])):
		with open("Python-3.5.1.tgz{}".format(i),'rb') as f:
			out_file.write(f.read())
