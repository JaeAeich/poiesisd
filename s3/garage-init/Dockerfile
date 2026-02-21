FROM dxflrs/garage:v2.2.0 AS garage

FROM busybox:1.36
COPY --from=garage /garage /garage

WORKDIR /app

COPY s3/garage-init/entrypoint.sh app/entrypoint.sh

ENTRYPOINT ["/bin/sh", "-ec", "app/entrypoint.sh"]
