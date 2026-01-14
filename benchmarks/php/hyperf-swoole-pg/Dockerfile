FROM phpswoole/swoole:php8.4-alpine

WORKDIR /opt/www

# Install extensions and dependencies
ADD --chmod=0755 https://github.com/mlocati/docker-php-extension-installer/releases/latest/download/install-php-extensions /usr/local/bin/

RUN apk add --no-cache libpq-dev curl \
    && install-php-extensions pdo pdo_pgsql pcntl

# Install composer (phpswoole image might not have it)
COPY --from=composer:latest /usr/bin/composer /usr/bin/composer

COPY composer.json .
RUN composer install --no-dev -o --ignore-platform-reqs \
    && rm -rf runtime/container

COPY . /opt/www

# Copy benchmarks data
COPY benchmarks_data /opt/www/benchmarks_data

ENV PORT=8080
ENV APP_ENV=prod
# ENV SCAN_CACHEABLE=true

EXPOSE 8080

HEALTHCHECK --interval=1s --timeout=3s --retries=30 CMD curl --fail http://127.0.0.1:8080/health || exit 1

ENTRYPOINT ["php", "/opt/www/bin/hyperf.php", "start"]
