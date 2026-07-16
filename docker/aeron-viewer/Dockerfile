FROM eclipse-temurin:17-jdk-jammy

ARG AERON_VERSION=1.44.1

RUN apt-get update \
    && apt-get install -y --no-install-recommends curl \
    && curl -fsSL -o /opt/aeron-all.jar \
        "https://repo1.maven.org/maven2/io/aeron/aeron-all/${AERON_VERSION}/aeron-all-${AERON_VERSION}.jar" \
    && apt-get purge -y curl \
    && apt-get autoremove -y \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY StreamViewer.java .

RUN javac -cp /opt/aeron-all.jar StreamViewer.java

ENV AERON_DIR=/data/aeron \
    AERON_CHANNEL=aeron:ipc \
    AERON_STREAM_ID=1001 \
    PORT=8080

EXPOSE 8080

CMD ["java", \
     "--add-opens", "java.base/sun.nio.ch=ALL-UNNAMED", \
     "--add-opens", "java.base/java.nio=ALL-UNNAMED", \
     "-cp", "/app:/opt/aeron-all.jar", \
     "StreamViewer"]
