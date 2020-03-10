Performance experiments

## DOS

Running `RunDOSBox.bat` 100 times from a batch file:

```bat
echo %time%
FOR /L %%n IN (1,1,100) DO CALL RunDOSBox.bat
echo %time%
```

For a simple hello world program:

```bas
10 PRINT "Hello, world!"
20 SYSTEM
```

Gives these numbers:

- start: 18:52:34,09
- end: 18:54:59,73
- diff: 2' 25'' .64 = 2 \* 60 + 25 + 0.64 sec = 145,64 sec
- avg: 1,4564sec = 1456,4msec

## Docker (outside)

Reminder: we build the image with:

```sh
docker build -t gwbasic -f Dockerfile.standalone .
```

Launching Docker 100 times from a batch file:

```bat
@ECHO OFF
echo %time%
FOR /L %%n IN (1,1,100) DO docker run --rm -v /c/Users/ngeor/Projects/github/dockerfiles/gwbasic:/app/basic gwbasic
echo %time%
```

- start: 19:12:22,46
- end: 19:14:12,43
- diff: 1' 49'' .97 = 1 \* 60 + 49 + 0.97 sec = 109,97 sec
- avg: 1,0997 sec = 1099,7msec

Interesting observation: the time is better even though we launch a new docker
image per iteration.

## Docker (inside)

First, we open a bash inside a container:

```sh
docker run --rm -it --entrypoint bash -v $PWD:/app/basic gwbasic
```

Then, we run the launcher script inside Docker 100 times:

```sh
root@0877df046f30:/app# date +%s%3N && for i in {1..100} ; do ./run-dos-box.sh basic/PROGRAM.BAS ; done && date +%s%3N
```

- start: 1583865891397 (epoch msec)
- end: 1583865959172
- diff: 67775 msec
- avg: 677,75 msec

## Apache

Reminder: we build the image with:

```sh
docker build -t gwbasic-httpd -f Dockerfile.httpd .
```

and start it with:

```sh
docker run --rm -d --name gwbasic-httpd -p 8080:80 -v $PWD/rest:/app/basic gwbasic-httpd
```

and stop it with:

```sh
docker stop gwbasic-httpd
```

While it is running, we'll create 100 new todo items:

```sh
date +%s%3N && for i in {1..100} ; do curl --data "hello $i" -H "Content-Type: text/plain" http://localhost:8080/api/todo ; done && date +%s%3N
```

But we can also write a script this time to use it again later (stored in
`perf.sh`):

```sh
#!/bin/bash
docker build -t gwbasic-httpd -f Dockerfile.httpd .
docker run --rm -d --name gwbasic-httpd -p 8080:80 -v $PWD/rest:/app/basic gwbasic-httpd
START=`date +%s%3N`
for i in {1..100} ; do curl --data "hello $i" -H "Content-Type: text/plain" http://localhost:8080/api/todo ; done
STOP=`date +%s%3N`
DIFF=$((STOP-START))
echo $DIFF
docker stop gwbasic-httpd
```

This gives 105304 msec for 100 POST calls, averaging at 1053,04msec.

Note that this script is more complicated than the hello world of the previous
examples.

## Summary

| Experiment       | Average duration (msec) |
| ---------------- | ----------------------: |
| DOS              |                  1456,4 |
| Docker (outside) |                  1099,7 |
| Docker (inside)  |                  677,75 |
| Apache           |                 1053,04 |