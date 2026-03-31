# Non Utf-8 to Utf-8 converter
## Purpose
RDS supports only UTF-8 file encoding. Sometimes you need to migrate old projects 
containing non UTF-8 files to 
RDS projects. This small utility helps. Non UTF-8 file is preserved with extension .bak after a conversion. 

## Usage
1. run `rb` in the directory
2. use built `nonutf8` to convert 'non-UTF-8' files to UTF-8

If a file in different than UTF-8 encoding contains a valuable information in the encoding,
then use some more powerful tool.