# Runtime Stage
FROM ruby:4.0-slim

# Install dependencies and tools
# libjemalloc2 for memory optimization
# libyaml-dev, pkg-config required for psych/rails
RUN apt-get update && apt-get install -y build-essential libpq-dev curl git libjemalloc2 libyaml-dev pkg-config && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy application code
COPY . .

# Install gems
RUN bundle config set --local without 'development test' && bundle install

# Copy benchmarks data (handled by runner usually, but good for standalone)
# COPY benchmarks_data /app/benchmarks_data 

ENV PORT=8080
ENV RAILS_ENV=production
ENV RAILS_LOG_TO_STDOUT=true
# Enable YJIT
ENV RUBY_YJIT_ENABLE=1
# Preload jemalloc (Generic path for Debian based systems)
ENV LD_PRELOAD=libjemalloc.so.2
ENV SECRET_KEY_BASE=1801202e03212871321782317823e23213213203210332857410321

# Create pid directory
RUN mkdir -p tmp/pids

# HEALTHCHECK
HEALTHCHECK --interval=1s --timeout=3s --retries=30 \
  CMD curl --fail http://localhost:8080/health || exit 1

# Start the server
CMD ["bundle", "exec", "puma", "-C", "config/puma.rb"]
