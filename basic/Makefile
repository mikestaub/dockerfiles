.PHONY: all run-perf perf docker-build-standalone docker-build-httpd clean

# TODO fix hardcoded unix PWD
PWD_UNIX = /c/Users/ngeor/Projects/github/dockerfiles/basic

# Location of binaries
GWBASIC_EXE = ./bin/GWBASIC.EXE
QBASIC_EXE = ./bin/QBASIC.EXE

LAUNCHER_EXE = ./basic-launcher-rust/target/release/basic-launcher-rust.exe
PERF_EXE = ./perf/target/release/perf.exe

# How many repetitions to run when doing performance testing
PERF_COUNT = 1

all: build-perf build-launcher build-docker-standalone build-docker-httpd build-docker-interpreter-httpd

#
# Launcher
#

build-launcher: $(LAUNCHER_EXE)

$(LAUNCHER_EXE): $(wildcard basic-launcher-rust/src/*.rs) basic-launcher-rust/Cargo.toml
	cd basic-launcher-rust && cargo build --release

clean-launcher:
	rm -rf basic-launcher-rust/target

#
# Performance
#

run-perf: build-launcher build-perf
	BLR_GWBASIC=$(GWBASIC_EXE) $(PERF_EXE) --count $(PERF_COUNT)

# make run-perf-qb
# make run-perf-qb PERF_COUNT=10 for more iterations
run-perf-qb: build-launcher build-perf
	BLR_QBASIC=$(QBASIC_EXE) BLR_BASIC_MODE=qbasic $(PERF_EXE) --count $(PERF_COUNT)

build-perf: $(PERF_EXE)

$(PERF_EXE): $(wildcard perf/src/*.rs) perf/Cargo.toml
	cd perf && cargo build --release

clean-perf:
	rm -rf perf/target

#
# Docker
#

build-docker-standalone: build-launcher
	docker build -t basic -f Dockerfile.standalone .

build-docker-httpd: build-launcher
	docker build -t basic-httpd -f Dockerfile.httpd .

build-docker-interpreter-httpd:
	docker build -t basic-interpreter-httpd -f Dockerfile.interpreter.httpd .


run-hello-dos: build-launcher
	BLR_GWBASIC=$(GWBASIC_EXE) $(LAUNCHER_EXE) ./basic/src/HELLO.BAS

run-hello-dos-qb: build-launcher
	BLR_QBASIC=$(QBASIC_EXE) $(LAUNCHER_EXE) ./basic/src/HELLOQB.BAS

run-hello-docker: build-docker-standalone
	docker.exe run --rm -v $(PWD_UNIX)/basic/src:/basic/src -v $(PWD_UNIX)/bin:/basic/bin basic HELLO.BAS

run-hello-docker-qb: build-docker-standalone
	docker.exe run --rm -v $(PWD_UNIX)/basic/src:/basic/src -v $(PWD_UNIX)/bin:/basic/bin -e BLR_BASIC_MODE=qbasic basic HELLOQB.BAS

run-httpd: build-docker-httpd
	docker run --rm -d --name basic-httpd -v $(PWD_UNIX)/basic/rest:/basic/src -v $(PWD_UNIX)/bin:/basic/bin -p 8080:80 basic-httpd
	curl http://localhost:8080/api/todo
	curl --data "Hello world" -H "Content-Type: text/plain" http://localhost:8080/api/todo
	curl http://localhost:8080/api/todo
	docker stop basic-httpd

run-httpd-qb: build-docker-httpd
	docker run --rm -d --name basic-httpd -v $(PWD_UNIX)/basic/rest-qb:/basic/src -v $(PWD_UNIX)/bin:/basic/bin -p 8080:80 basic-httpd
	curl -v http://localhost:8080/api/todo
	curl -v --data "Hello world" -H "Content-Type: text/plain" http://localhost:8080/api/todo
	curl -v http://localhost:8080/api/todo
	docker stop basic-httpd

start-httpd-foreground: build-docker-httpd
	docker run -e BLR_NO_CLEANUP=1 --rm --name basic-httpd -v $(PWD_UNIX)/basic/rest:/basic/src -v $(PWD_UNIX)/bin:/basic/bin -p 8080:80 basic-httpd

start-httpd-foreground-qb: build-docker-httpd
	docker run -e BLR_NO_CLEANUP=1 --rm --name basic-httpd -v $(PWD_UNIX)/basic/rest-qb:/basic/src -v $(PWD_UNIX)/bin:/basic/bin -p 8080:80 basic-httpd

test: run-hello-dos run-hello-dos-qb run-hello-docker run-hello-docker-qb run-httpd run-httpd-qb

start-interpreter-httpd-foreground: build-docker-interpreter-httpd
	docker run --rm --name basic-interpreter-httpd -v $(PWD_UNIX)/basic/rest-qb-direct:/basic/src -p 8080:80 basic-interpreter-httpd

clean: clean-basic-launcher-rust clean-perf
