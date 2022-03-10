#!/bin/bash
rmdir $2
mkdir -p $2
find $1/* | parallel --jobs 4 --progress ./compress.sh "{}" $2
