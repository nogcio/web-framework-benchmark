FROM dunglas/frankenphp

RUN install-php-extensions \
    pcntl \
    pdo_pgsql \
    pgsql \
    opcache \
    zip

WORKDIR /app

COPY composer.json ./
COPY php.ini /usr/local/etc/php/conf.d/99-app.ini

ENV COMPOSER_ALLOW_SUPERUSER=1
RUN curl -sS https://getcomposer.org/installer | php -- --install-dir=/usr/local/bin --filename=composer \
    && composer install --no-dev --no-interaction --prefer-dist --optimize-autoloader --classmap-authoritative --no-scripts

COPY . .

# Setup storage
RUN mkdir -p storage/framework/views storage/framework/cache storage/framework/sessions \
    && mkdir -p storage/logs \
    && mkdir -p bootstrap/cache \
    && mkdir -p database \
    && chmod -R 777 storage bootstrap/cache database

RUN composer dump-autoload --optimize --classmap-authoritative

# Install Octane
RUN php artisan octane:install --server=frankenphp

# Static files
COPY benchmarks_data /app/public/files

# Setup Laravel optimization
# config:cache is omitted to allow runtime environment variables (like DB_HOST) to take effect
# We will run these at runtime for maximum flexibility + performance
RUN php artisan route:cache && php artisan view:cache

COPY Caddyfile /etc/caddy/Caddyfile
COPY php.ini /usr/local/etc/php/conf.d/99-app.ini


ENV PORT=8080
ENV APP_ENV=production
ENV APP_DEBUG=false
ENV SESSION_DRIVER=array
ENV LOG_CHANNEL=stderr
ENV APP_KEY=base64:yS9jE6O6eeTcRrZ73Oa3mKxa4V+P9cyT2GI0XX2Lg8M=

HEALTHCHECK --interval=1s --timeout=3s --retries=30 \
  CMD curl --fail http://127.0.0.1:8080/health || exit 1

CMD ["sh", "-c", "php artisan config:cache && php artisan route:cache && php artisan view:cache && php artisan octane:start --server=frankenphp --host=0.0.0.0 --port=8080 --workers=auto"]
