# dockerfiles

> A home for my various custom Docker images.

[![Build Status](https://travis-ci.org/ngeor/dockerfiles.svg?branch=master)](https://travis-ci.org/ngeor/dockerfiles)

**Warning** I am not currently using any of the images listed below, so they
are most likely not maintained.

## Images

| Image                           | Base image                  | Main features                    | Extra                                                     | Pulls                                                                                        |
| ------------------------------- | --------------------------- | -------------------------------- | --------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| [awscli]                        | Python alpine               | AWS CLI, zip, git                | zip, git                                                  | ![Docker Pulls](https://img.shields.io/docker/pulls/ngeor/awscli.svg)                        |
| [awscli-terraform]              | Python alpine               | AWS CLI, terraform               | zip, git, ca-certificates, openssl                        | ![Docker Pulls](https://img.shields.io/docker/pulls/ngeor/awscli-terraform.svg)              |
| [az-helm-kubectl-terraform]     | Azure CLI                   | kubectl, helm, terraform         | bash, curl, git, ca-certificates, initialized helm client | ![Docker Pulls](https://img.shields.io/docker/pulls/ngeor/az-helm-kubectl-terraform.svg)     |
| [helm-kubectl-terraform]        | Alpine                      | kubectl, helm, terraform         | bash, curl, git, ca-certificates, initialized helm client | ![Docker Pulls](https://img.shields.io/docker/pulls/ngeor/helm-kubectl-terraform.svg)        |
| [jdk-helm-kubectl-terraform]    | Maven 3.6 JDK 11 slim       | kubectl, helm, terraform         | unzip, ant, gradle, initialized helm client               | ![Docker Pulls](https://img.shields.io/docker/pulls/ngeor/jdk-helm-kubectl-terraform.svg)    |
| [maven-awscli]                  | Maven 3.6 JDK 11 slim       | AWS CLI                          | python, pip, nc                                           | ![Docker Pulls](https://img.shields.io/docker/pulls/ngeor/maven-awscli.svg)                  |
| [node-chrome]                   | Alpeware Chrome Headless 77 | nodeJS 10                        | curl, build-essential                                     | ![Docker Pulls](https://img.shields.io/docker/pulls/ngeor/node-chrome.svg)                   |
| [node-firefox]                  | nodeJS 10 Jessie            | Firefox                          |                                                           | ![Docker Pulls](https://img.shields.io/docker/pulls/ngeor/node-firefox.svg)                  |
| [python-helm-kubectl-terraform] | Python alpine               | kubectl, helm, terraform         | bash, curl, git, ca-certificates, initialized helm client | ![Docker Pulls](https://img.shields.io/docker/pulls/ngeor/python-helm-kubectl-terraform.svg) |
| [ruby-helm-kubectl-terraform]   | Ruby 2.5.3 alpine           | kubectl, helm, terraform         | bash, curl, git, ca-certificates, initialized helm client | ![Docker Pulls](https://img.shields.io/docker/pulls/ngeor/ruby-helm-kubectl-terraform.svg)   |
| [swagger-to-diagram]            | OpenJDK JRE 11 slim         | custom swagger to diagram script | curl, python, graphviz                                    | ![Docker Pulls](https://img.shields.io/docker/pulls/ngeor/swagger-to-diagram.svg)            |

## \*-helm-kubectl-terraform

Docker images that include helm, kubectl, and terraform.

- Inspired by [dtzar/helm-kubectl](https://github.com/dtzar/helm-kubectl)
- Includes `terraform`
- Current versions: `kubectl` v1.17.0, `helm` v2.16.1, `terraform` 0.12.18

[awscli]: https://hub.docker.com/r/ngeor/awscli/
[awscli-terraform]: https://hub.docker.com/r/ngeor/awscli-terraform/
[az-helm-kubectl-terraform]: https://hub.docker.com/r/ngeor/az-helm-kubectl-terraform/
[helm-kubectl-terraform]: https://hub.docker.com/r/ngeor/helm-kubectl-terraform/
[jdk-helm-kubectl-terraform]: https://hub.docker.com/r/ngeor/jdk-helm-kubectl-terraform/
[maven-awscli]: https://hub.docker.com/r/ngeor/maven-awscli/
[node-chrome]: https://hub.docker.com/r/ngeor/node-chrome/
[node-firefox]: https://hub.docker.com/r/ngeor/node-firefox/
[python-helm-kubectl-terraform]: https://hub.docker.com/r/ngeor/python-helm-kubectl-terraform/
[ruby-helm-kubectl-terraform]: https://hub.docker.com/r/ngeor/ruby-helm-kubectl-terraform/
[swagger-to-diagram]: https://hub.docker.com/r/ngeor/swagger-to-diagram/

## basic

Docker images that launch GW-Basic and QBasic using DOSBox.

See the [related README](https://github.com/ngeor/dockerfiles/tree/master/basic) for more information.

## dockerv

A utility for easier working with Docker volumes on Windows 10 Home.

See the [related README](https://github.com/ngeor/dockerfiles/tree/master/dockerv) for more information.

## Building and releasing

To avoid rebuilding everything, images will only be built if:

- their folder has changes
- there is a tag in the format of `vX.Y.Z-folder`

If an image is built, it will be released when:

- building on the master branch, in which case it is tagged as `latest`
- building on a tag `vX.Y.Z-folder`, in which case it is tagged as `vX.Y.Z`
