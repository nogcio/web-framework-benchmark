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
        if (!is_array($orders)) {
             return $response->json([]);
        }

        $processedOrders = 0;
        $results = [];
        $categoryStats = [];

        foreach ($orders as $order) {
            if (isset($order['status']) && $order['status'] === 'completed') {
                $processedOrders++;

                $country = $order['country'] ?? '';
                if (!isset($results[$country])) {
                    $results[$country] = 0;
                }
                $results[$country] += $order['amount'] ?? 0;

                if (isset($order['items']) && is_array($order['items'])) {
                    foreach ($order['items'] as $item) {
                        $category = $item['category'] ?? '';
                        if (!isset($categoryStats[$category])) {
                            $categoryStats[$category] = 0;
                        }
                        $categoryStats[$category] += $item['quantity'] ?? 0;
                    }
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
