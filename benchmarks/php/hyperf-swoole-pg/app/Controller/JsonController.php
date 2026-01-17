<?php

declare(strict_types=1);

namespace App\Controller;

use Hyperf\HttpServer\Contract\RequestInterface;
use Hyperf\HttpServer\Contract\ResponseInterface;

class JsonController
{
    public function aggregate(RequestInterface $request, ResponseInterface $response)
    {
        $orders = $request->getParsedBody();

        $processedOrders = 0;
        $results = [];
        $categoryStats = [];

        foreach ($orders as $order) {
            if ($order['status'] === 'completed') {
                $processedOrders++;

                $country = $order['country'];
                $results[$country] = ($results[$country] ?? 0) + $order['amount'];

                foreach ($order['items'] as $item) {
                    $category = $item['category'];
                    $categoryStats[$category] = ($categoryStats[$category] ?? 0) + $item['quantity'];
                }
            }
        }

        return $response->json([
            'processedOrders' => $processedOrders,
            'results' => $results,
            'categoryStats' => $categoryStats,
        ]);
    }
}
