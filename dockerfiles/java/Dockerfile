# Java Execution Environment - Optimized for Cold Start
FROM eclipse-temurin:17-jdk-alpine

# Set environment variables for performance
ENV JAVA_TOOL_OPTIONS="-XX:+UseContainerSupport -XX:MaxRAMPercentage=75.0 -XX:+TieredCompilation -XX:TieredStopAtLevel=1"

WORKDIR /code

# Install JUnit at build time (not runtime) and cleanup
RUN apk add --no-cache wget && \
    wget -q https://repo1.maven.org/maven2/junit/junit/4.13.2/junit-4.13.2.jar -P /opt/ && \
    wget -q https://repo1.maven.org/maven2/org/hamcrest/hamcrest-core/1.3/hamcrest-core-1.3.jar -P /opt/ && \
    apk del wget && \
    rm -rf /var/cache/apk/*

ENV CLASSPATH=/opt/junit-4.13.2.jar:/opt/hamcrest-core-1.3.jar

# Copy and compile runner at build time
COPY dockerfiles/java/Runner.java /Runner.java
RUN javac /Runner.java

# Create non-root user for security
RUN adduser -D -u 1000 optimus && \
    chown -R optimus:optimus /code

USER optimus

# Minimal entrypoint (no shell overhead)
ENTRYPOINT ["java", "Runner"]
