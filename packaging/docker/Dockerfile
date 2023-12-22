FROM --platform=$BUILDPLATFORM debian:12.4-slim as build
ARG TARGETPLATFORM
ARG BUILDPLATFORM
COPY ./copy-binary.sh /opt/searproxy/scripts/copy-binary.sh
RUN chmod ug+x /opt/searproxy/scripts/copy-binary.sh
RUN /opt/searproxy/scripts/copy-binary.sh

FROM alpine:3.19.0
COPY --from=build /usr/local/bin/searproxy /usr/local/bin/searproxy
CMD [ "/usr/local/bin/searproxy" ]
