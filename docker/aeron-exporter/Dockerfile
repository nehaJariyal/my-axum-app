FROM eclipse-temurin:8-jdk-jammy

RUN apt-get update \
    && apt-get install -y --no-install-recommends git \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build
RUN git clone --depth 1 https://github.com/fairtide/aeron-prometheus-stat.git . \
    && ./gradlew fatJar --no-daemon

FROM eclipse-temurin:17-jre-jammy

COPY --from=0 /build/build/libs/aeron-prometheus-stats-all-1.0-SNAPSHOT.jar /opt/aeron-exporter.jar

ENV AERON_DIR=/data/aeron

EXPOSE 50051

CMD ["java", \
     "-Daeron.dir=/data/aeron", \
     "-cp", "/opt/aeron-exporter.jar", \
     "co.fairtide.prometheus_aeron_stat.PrometheusAeronStat", \
     "-port", "50051", "-delay", "5000"]
