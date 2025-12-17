# Python Execution Environment - Optimized for Cold Start
FROM python:3.11-slim

# Set environment variables for performance
ENV PYTHONUNBUFFERED=1 \
    PYTHONDONTWRITEBYTECODE=1 \
    PIP_NO_CACHE_DIR=1 \
    PIP_DISABLE_PIP_VERSION_CHECK=1

WORKDIR /code

# Install common packages (pre-installed at build time, not runtime)
RUN pip install --no-cache-dir \
    pytest==7.4.3 \
    numpy==1.26.2 \
    requests==2.31.0

# Copy runner script
COPY dockerfiles/python/runner.py /runner.py

# Create non-root user for security
RUN useradd -m -u 1000 optimus && \
    chown -R optimus:optimus /code

USER optimus

# Minimal entrypoint (no shell overhead)
ENTRYPOINT ["python", "-u", "/runner.py"]
