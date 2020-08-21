# swagger-to-diagram

A Docker image that can be used to generate UML diagrams from Swagger
definitions

Run with:

```bash
docker run --rm -v $PWD:/data \
  ngeor/swagger-to-diagram swagger2png.sh \
  swagger-input.yml diagram-output.png
```

To generate a .puml file:

```bash
docker run --rm -v $PWD:/data \
  ngeor/swagger-to-diagram swagger2uml.sh \
  swagger-input.yml diagram-output.puml
```

You can also publish the page to Confluence Cloud with a different script which
is baked into the image:

```
docker run --rm -v $PWD:/data \
  ngeor/swagger-to-diagram put-confluence-attachement.sh \
  -u confuence-username:confluence-password \
  --filename path-to-file.png \
  --comment "comment for describing changes" \
  --domain confluence-cloud-domain-without-atlassian.net \
  --content-id "the id of the page to put the attachment in"
```
