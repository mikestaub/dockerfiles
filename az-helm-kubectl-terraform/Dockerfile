FROM microsoft/azure-cli

# Latest kubectl is here: https://storage.googleapis.com/kubernetes-release/release/stable.txt
ARG KUBECTL_VERSION="v1.17.0"
ARG HELM_VERSION="v2.16.1"
ARG TERRAFORM_VERSION="0.12.18"

RUN apk add --no-cache ca-certificates bash curl git \
  && wget -q https://storage.googleapis.com/kubernetes-release/release/${KUBECTL_VERSION}/bin/linux/amd64/kubectl -O /usr/local/bin/kubectl \
  && chmod +x /usr/local/bin/kubectl \
  && wget -q https://storage.googleapis.com/kubernetes-helm/helm-${HELM_VERSION}-linux-amd64.tar.gz -O - | tar -xzO linux-amd64/helm > /usr/local/bin/helm \
  && chmod +x /usr/local/bin/helm \
  && wget -q -O terraform.zip https://releases.hashicorp.com/terraform/${TERRAFORM_VERSION}/terraform_${TERRAFORM_VERSION}_linux_amd64.zip \
  && unzip terraform.zip -d /usr/local/bin \
  && rm -rf terraform.zip \
  && helm init -c

WORKDIR /config
CMD bash
