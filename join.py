import sys

with open("sublime.deb","ab") as out_file:
	for i in range(0,int(sys.argv[1])):
		with open("sublime-text_build-3103_amd64.deb{}".format(i),'rb') as f:
			out_file.write(f.read())
