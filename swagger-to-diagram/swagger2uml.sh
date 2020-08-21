#!/bin/bash

python3 $(dirname $0)/swagger_to_uml.py $1 > $2
