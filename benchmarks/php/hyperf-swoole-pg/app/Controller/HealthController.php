<?php

declare(strict_types=1);

namespace App\Controller;

use Hyperf\HttpServer\Contract\ResponseInterface;

class HealthController
{
    public function index(ResponseInterface $response)
    {
        // Simple DB Check
        try {
            if (\Hyperf\DbConnection\Db::connection()->select('SELECT 1')) {
                 return $response->raw('OK');
            }
            return $response->withStatus(500)->raw('DB Check failed');
        } catch (\Throwable $e) {
            $msg = "Health check failed: " . $e->getMessage();
            fwrite(STDERR, $msg . "\n");
            return $response->withStatus(500)->raw($msg);
        }
    }
}
