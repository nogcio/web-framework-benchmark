<?php

declare(strict_types=1);

use Hyperf\HttpServer\Router\Router;

Router::addRoute(['GET', 'HEAD'], '/plaintext', 'App\Controller\IndexController@index');
Router::get('/health', 'App\Controller\HealthController@index');
Router::post('/json/aggregate', 'App\Controller\JsonController@aggregate');
Router::get('/db/user-profile/{email}', 'App\Controller\DbController@complex');

Router::get('/favicon.ico', function () {
    return '';
});
