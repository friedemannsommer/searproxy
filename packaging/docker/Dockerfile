FROM --platform=$BUILDPLATFORM debian:12.4-slim as build
ARG TARGETPLATFORM
ARG BUILDPLATFORM
COPY ./copy-binary.sh /opt/searproxy/scripts/copy-binary.sh
COPY ./artifact-binaries /opt/searproxy/binaries
RUN chmod ug+x /opt/searproxy/scripts/copy-binary.sh
RUN /opt/searproxy/scripts/copy-binary.sh

FROM alpine:3.19.0
ARG UID=1000
ARG GID=1000
COPY --from=build /usr/local/bin/searproxy /usr/local/bin/searproxy
RUN addgroup -g "${GID}" searproxy \
  && adduser -u "${UID}" -G "${GID}" searproxy
USER searproxy
CMD [ "/usr/local/bin/searproxy" ]
