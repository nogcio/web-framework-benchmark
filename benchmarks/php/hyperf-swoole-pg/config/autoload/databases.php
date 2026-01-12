<?php

declare(strict_types=1);

return [
    'default' => [
        'driver' => 'pgsql',
        'host' => \Hyperf\Support\env('DB_HOST', 'localhost'),
        'port' => \Hyperf\Support\env('DB_PORT', 5432),
        'database' => \Hyperf\Support\env('DB_NAME', 'hello_world'),
        'username' => \Hyperf\Support\env('DB_USER', 'user'),
        'password' => \Hyperf\Support\env('DB_PASSWORD', 'password'),
        'charset' => 'utf8',
        'collation' => 'utf8_unicode_ci',
        'prefix' => '',
        'pool' => [
            'min_connections' => 1,
            'max_connections' => (int)\Hyperf\Support\env('DB_POOL_SIZE', 256),
            'connect_timeout' => 10.0,
            'wait_timeout' => 3.0,
            'heartbeat' => -1,
            'max_idle_time' => 60.0,
        ],
    ],
];
