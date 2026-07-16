FROM eclipse-temurin:17-jre-jammy

ARG AERON_VERSION=1.44.1

RUN apt-get update \
    && apt-get install -y --no-install-recommends curl \
    && curl -fsSL -o /opt/aeron-all.jar \
        "https://repo1.maven.org/maven2/io/aeron/aeron-all/${AERON_VERSION}/aeron-all-${AERON_VERSION}.jar" \
    && apt-get purge -y curl \
    && apt-get autoremove -y \
    && rm -rf /var/lib/apt/lists/*

ENV AERON_DIR=/data/aeron

CMD ["java", \
     "--add-opens", "java.base/sun.nio.ch=ALL-UNNAMED", \
     "--add-opens", "java.base/java.nio=ALL-UNNAMED", \
     "-Daeron.dir=/data/aeron", \
     "-Daeron.dir.delete.on.start=false", \
     "-cp", "/opt/aeron-all.jar", \
     "io.aeron.driver.MediaDriver"]
