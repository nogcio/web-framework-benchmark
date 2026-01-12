FROM phpswoole/swoole:php8.2-alpine

WORKDIR /opt/www

COPY . /opt/www

# Install extensions and dependencies
RUN apk add --no-cache libpq-dev curl \
    && docker-php-ext-install pdo pdo_pgsql pcntl

# Install composer (phpswoole image might not have it)
COPY --from=composer:latest /usr/bin/composer /usr/bin/composer

RUN composer install --no-dev -o \
    && rm -rf runtime/container

# Copy benchmarks data
COPY benchmarks_data /opt/www/benchmarks_data

ENV PORT=8080
ENV APP_ENV=prod
# ENV SCAN_CACHEABLE=true

EXPOSE 8080

HEALTHCHECK --interval=5s --timeout=3s --retries=3 CMD curl --fail http://127.0.0.1:8080/health || exit 1

ENTRYPOINT ["php", "/opt/www/bin/hyperf.php", "start"]
