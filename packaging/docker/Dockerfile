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
RUN addgroup --gid "${GID}" searproxy \
  && adduser --uid "${UID}" --ingroup searproxy --shell /sbin/nologin --disabled-password --no-create-home searproxy
USER searproxy
COPY --from=build --chown=searproxy /usr/local/bin/searproxy /usr/local/bin/searproxy
CMD [ "/usr/local/bin/searproxy" ]
