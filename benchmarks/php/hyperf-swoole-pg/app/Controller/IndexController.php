<?php

declare(strict_types=1);

namespace App\Controller;

use Hyperf\HttpServer\Contract\RequestInterface;
use Hyperf\HttpServer\Contract\ResponseInterface;
use Hyperf\DbConnection\Db;
use Hyperf\Context\ApplicationContext;

class IndexController
{
    public function index()
    {
        return 'Hello, World!';
    }
}
